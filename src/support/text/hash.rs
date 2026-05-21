use base64::Engine as _;
use base64::engine::general_purpose;
use blake3::Hasher;

pub fn blake3_b64u(parts: &[&str]) -> String {
	let mut hasher = Hasher::new();

	for part in parts {
		hasher.update(part.as_bytes());
	}

	let hash = hasher.finalize();

	general_purpose::URL_SAFE_NO_PAD.encode(hash.as_bytes())
}
