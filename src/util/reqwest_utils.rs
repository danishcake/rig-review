use reqwest::Certificate;

use crate::{config::NetworkConfig, error::RigReviewError};

/// Loads the certificates from the network config
pub fn load_certificates(
    network_config: &NetworkConfig,
) -> Result<Vec<Certificate>, RigReviewError> {
    network_config
        .extra_ca
        .iter()
        .map(|cert_path| {
            let cert_blob = std::fs::read(cert_path)?;
            Ok(Certificate::from_pem(&cert_blob)?)
        })
        .collect::<Result<Vec<Certificate>, RigReviewError>>()
}

/// Loads the certificates from the network config as a pem bundle
pub fn load_certificates_as_pem_bundle(
    network_config: &NetworkConfig,
) -> Result<Vec<u8>, RigReviewError> {
    let bundle: Vec<u8> = network_config.extra_ca.iter().try_fold(
        Vec::new(),
        |mut acc, file| -> Result<Vec<u8>, RigReviewError> {
            let cert_blob = std::fs::read(file)?;
            acc.extend(&cert_blob);

            if cert_blob.last() != Some(&b'\n') {
                acc.push(b'\n');
            }
            Ok(acc)
        },
    )?;

    Ok(bundle)
}
