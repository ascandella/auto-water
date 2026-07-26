use defmt::info;
use embassy_net::tcp::TcpSocket;
use embedded_storage::Storage;
use esp_bootloader_esp_idf::ota::OtaImageState;
use esp_bootloader_esp_idf::ota_updater::OtaUpdater;
use esp_bootloader_esp_idf::partitions::PARTITION_TABLE_MAX_LEN;
use esp_storage::FlashStorage;

pub const OTA_PASSWORD: &str = match option_env!("OTA_PASSWORD") {
    Some(p) => p,
    None => "auto-water-ota",
};

const WRITE_CHUNK_SIZE: usize = 4096;

#[derive(Debug)]
pub enum OtaError {
    UpdaterInit,
    NoNextPartition,
    SocketRead,
    FlashWrite,
    Activate,
}

impl defmt::Format for OtaError {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            OtaError::UpdaterInit => defmt::write!(fmt, "UpdaterInit"),
            OtaError::NoNextPartition => defmt::write!(fmt, "NoNextPartition"),
            OtaError::SocketRead => defmt::write!(fmt, "SocketRead"),
            OtaError::FlashWrite => defmt::write!(fmt, "FlashWrite"),
            OtaError::Activate => defmt::write!(fmt, "Activate"),
        }
    }
}

pub fn check_auth(auth_header: Option<&[u8]>) -> bool {
    match auth_header {
        Some(value) => value == OTA_PASSWORD.as_bytes(),
        None => false,
    }
}

pub async fn perform_ota(
    flash: &mut FlashStorage<'static>,
    socket: &mut TcpSocket<'_>,
    content_length: usize,
    initial_body: &[u8],
) -> Result<(), OtaError> {
    let mut buffer = [0u8; PARTITION_TABLE_MAX_LEN];
    let mut ota = OtaUpdater::new(flash, &mut buffer).map_err(|_| OtaError::UpdaterInit)?;

    let (mut next_partition, part_type) = ota
        .next_partition()
        .map_err(|_| OtaError::NoNextPartition)?;
    info!("OTA: writing to {:?}", part_type);

    let mut write_buf = [0u8; WRITE_CHUNK_SIZE];
    let mut total_written = 0usize;
    let mut initial_pos = 0usize;

    while total_written < content_length {
        let remaining = content_length - total_written;
        let chunk_size = core::cmp::min(WRITE_CHUNK_SIZE, remaining);

        let mut filled = 0usize;

        let from_initial = core::cmp::min(initial_body.len() - initial_pos, chunk_size);
        if from_initial > 0 {
            write_buf[..from_initial]
                .copy_from_slice(&initial_body[initial_pos..initial_pos + from_initial]);
            filled = from_initial;
            initial_pos += from_initial;
        }

        while filled < chunk_size {
            let n = socket
                .read(&mut write_buf[filled..chunk_size])
                .await
                .map_err(|_| OtaError::SocketRead)?;
            if n == 0 {
                return Err(OtaError::SocketRead);
            }
            filled += n;
        }

        next_partition
            .write(total_written as u32, &write_buf[..chunk_size])
            .map_err(|_| OtaError::FlashWrite)?;

        total_written += chunk_size;
    }

    info!("OTA: activating new partition");
    ota.activate_next_partition()
        .map_err(|_| OtaError::Activate)?;
    ota.set_current_ota_state(OtaImageState::New)
        .map_err(|_| OtaError::Activate)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_accepts_correct_password() {
        assert!(check_auth(Some(b"Bearer auto-water-ota")));
    }

    #[test]
    fn auth_rejects_wrong_password() {
        assert!(!check_auth(Some(b"Bearer wrong-password")));
    }

    #[test]
    fn auth_rejects_none() {
        assert!(!check_auth(None));
    }

    #[test]
    fn auth_rejects_empty_header() {
        assert!(!check_auth(Some(b"")));
    }

    #[test]
    fn auth_rejects_bearer_without_token() {
        assert!(!check_auth(Some(b"Bearer ")));
    }

    #[test]
    fn auth_rejects_non_bearer_scheme() {
        assert!(!check_auth(Some(b"Basic dXNlcjpwYXNz")));
    }

    #[test]
    fn default_password_is_not_empty() {
        assert!(!OTA_PASSWORD.is_empty());
    }
}
