mod avro;

use anyhow::Result;
use redpanda_transform_sdk::*;
use sha2::{Digest, Sha256};

/// SAFETY: The runtime environment must be single-threaded WASM.
#[cfg(target_family = "wasm")]
#[global_allocator]
static ALLOCATOR: talc::TalckWasm = unsafe { talc::TalckWasm::new_global() };

fn main() {
    on_record_written(my_transform);
}

fn my_transform(event: WriteEvent, writer: &mut RecordWriter) -> Result<()> {
    let value = event.record.value().unwrap_or_default();
    let mut interop: avro::Interop = match serde_json::from_slice(value) {
        Ok(i) => i,
        // OMB sends some dummy messages sometimes, so just skip those
        Err(_) => return Ok(()),
    };
    redact(&mut interop);
    let redacted = serde_json::to_vec(&interop)?;
    writer.write(BorrowedRecord::new_with_headers(
        event.record.key(),
        Some(&redacted),
        event.record.headers().to_vec(),
    ))?;
    Ok(())
}

fn redact(interop: &mut avro::Interop) {
    interop.string_field = Default::default();
    interop.int_field = i32_hash(interop.int_field);
    interop.long_field = i64_hash(interop.long_field);
    interop.float_field = f32_hash(interop.float_field);
    interop.double_field = f64_hash(interop.double_field);
}

fn i32_hash(v: i32) -> i32 {
    let hash = Sha256::digest(v.to_be_bytes());
    let (int_bytes, _) = hash.split_at(std::mem::size_of::<i32>());
    i32::from_be_bytes(int_bytes.try_into().unwrap())
}

fn i64_hash(v: i64) -> i64 {
    let hash = Sha256::digest(v.to_be_bytes());
    let (int_bytes, _) = hash.split_at(std::mem::size_of::<i64>());
    i64::from_be_bytes(int_bytes.try_into().unwrap())
}

fn f32_hash(v: f32) -> f32 {
    let hash = Sha256::digest(v.to_be_bytes());
    let (int_bytes, _) = hash.split_at(std::mem::size_of::<f32>());
    f32::from_be_bytes(int_bytes.try_into().unwrap())
}

fn f64_hash(v: f64) -> f64 {
    let hash = Sha256::digest(v.to_be_bytes());
    let (int_bytes, _) = hash.split_at(std::mem::size_of::<f64>());
    f64::from_be_bytes(int_bytes.try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use crate::avro;
    use anyhow::Result;
    use serde_json;
    #[test]
    fn round_trip_interop() -> Result<()> {
        let value = std::fs::read_to_string("../../payload/payload-1Kb.json")?;
        let interop: avro::Interop = serde_json::from_slice(value.as_bytes())?;
        println!("{:?}", interop);
        Ok(())
    }
}
