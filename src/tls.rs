use std::path::{Path, PathBuf};

use reqwest::Certificate;

const CUSTOM_ROOT_CERT_DIR: &'static str = "garrysmod/tls_certificates";

#[derive(thiserror::Error, Debug)]
pub enum CertificateError {
	#[error("invalid certificate extension: {0:?}")]
	InvalidExtension(Option<String>),

	#[error("reqwest error: {0:#?}")]
	Other(#[from] reqwest::Error),

	#[error("io error: {0:?}")]
	IoError(#[from] std::io::Error),
}

fn get_cert_from_file<P: AsRef<Path>>(path: P) -> Result<Certificate, CertificateError> {
	let path = path.as_ref();

	let ext = match path.extension().and_then(|ext| ext.to_str()) {
		Some(ext @ ("pem" | "der")) => ext,
		_ => {
			return Err(CertificateError::InvalidExtension(
				path.extension().map(|ext| ext.to_string_lossy().into_owned()),
			))
		}
	};

	let cert = std::fs::read(path)?;

	Ok(match ext {
		"pem" => Certificate::from_pem(&cert)?,
		"der" => Certificate::from_der(&cert)?,
		_ => unreachable!(),
	})
}

pub fn get_loadable_certificates() -> Result<Vec<Certificate>, std::io::Error> {
	if !PathBuf::from(CUSTOM_ROOT_CERT_DIR).exists() {
		return Ok(vec![]);
	}

	Ok(
		std::fs::read_dir(CUSTOM_ROOT_CERT_DIR)?
			.filter_map(|entry| entry.ok())
			.filter_map(|entry| match get_cert_from_file(entry.path()) {
				Ok(cert) => Some(cert),
				Err(err) => {
					eprintln!("[gmsv_reqwest] Error loading certificate \"{:?}\": {}", entry.path().file_name(), err);
					None
				}
			})
			.collect::<Vec<Certificate>>()
	)
}
