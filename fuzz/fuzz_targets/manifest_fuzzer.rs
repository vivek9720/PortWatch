#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    portwatch::fuzz_manifest(data);
});
