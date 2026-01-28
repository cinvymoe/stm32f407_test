//! SD card reader module for firmware file access
//!
//! This module provides an async interface to read firmware files from SD cards
//! using the SDIO interface with minimal FAT32 support for bootloader use.
//!
//! # Requirements
//! - 需求 3.1.1: SDIO 接口配置（4-bit 数据总线模式）
//! - 需求 3.3.1: 固件文件查找和读取
//! - 需求 3.4.2: 固件读取
//! - 需求 3.5.3: 超时保护（5 秒初始化超时）

use crate::config::{FIRMWARE_FILENAME, SD_INIT_TIMEOUT};
use crate::error::SdError;
use embassy_stm32::sdmmc::{DataBlock, Instance, Sdmmc, SdmmcDma};
use embassy_stm32::time::Hertz;
use embassy_time::{with_timeout, Duration};

/// SD card reader structure
///
/// Provides async methods to initialize SDIO interface and read firmware files
/// from FAT32 formatted SD cards.
pub struct SdCardReader<'d, T: Instance, D: SdmmcDma<T>> {
    /// SDIO/SDMMC peripheral
    sdmmc: Sdmmc<'d, T, D>,
    /// FAT32 filesystem information (initialized after mount)
    fat_info: Option<Fat32Info>,
}

/// FAT32 filesystem information
#[derive(Debug, Clone, Copy)]
struct Fat32Info {
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    num_fats: u8,
    fat_size: u32,
    root_cluster: u32,
    first_data_sector: u32,
}

impl Fat32Info {
    /// Convert cluster number to first sector number
    fn cluster_to_sector(&self, cluster: u32) -> u32 {
        self.first_data_sector + (cluster - 2) * self.sectors_per_cluster as u32
    }

    /// Get FAT sector for a given cluster
    fn fat_sector(&self, cluster: u32) -> u32 {
        self.reserved_sectors as u32 + (cluster * 4) / self.bytes_per_sector as u32
    }

    /// Get offset within FAT sector for a given cluster
    fn fat_offset(&self, cluster: u32) -> usize {
        ((cluster * 4) % self.bytes_per_sector as u32) as usize
    }
}

/// FAT32 Directory Entry (32 bytes)
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct DirEntry {
    name: [u8; 11],
    attr: u8,
    nt_reserved: u8,
    create_time_tenth: u8,
    create_time: u16,
    create_date: u16,
    last_access_date: u16,
    first_cluster_hi: u16,
    write_time: u16,
    write_date: u16,
    first_cluster_lo: u16,
    file_size: u32,
}

impl DirEntry {
    const ATTR_LONG_NAME: u8 = 0x0F;
    const ATTR_DIRECTORY: u8 = 0x10;
    const ATTR_VOLUME_ID: u8 = 0x08;

    fn is_valid(&self) -> bool {
        self.name[0] != 0x00 && self.name[0] != 0xE5
    }

    fn is_long_name(&self) -> bool {
        self.attr == Self::ATTR_LONG_NAME
    }

    fn is_directory(&self) -> bool {
        (self.attr & Self::ATTR_DIRECTORY) != 0
    }

    fn is_volume_id(&self) -> bool {
        (self.attr & Self::ATTR_VOLUME_ID) != 0
    }

    fn first_cluster(&self) -> u32 {
        ((self.first_cluster_hi as u32) << 16) | (self.first_cluster_lo as u32)
    }

    /// Check if this entry matches the given filename (8.3 format)
    fn matches_name(&self, name: &str) -> bool {
        if self.is_long_name() || self.is_directory() || self.is_volume_id() {
            return false;
        }

        // Convert filename to FAT 8.3 format
        let mut fat_name = [b' '; 11];

        // Manual uppercase conversion for no_std
        let mut name_bytes = [0u8; 32];
        let mut len = 0;
        for (i, b) in name.bytes().enumerate() {
            if i >= 32 {
                break;
            }
            name_bytes[i] = if b >= b'a' && b <= b'z' {
                b - 32 // Convert to uppercase
            } else {
                b
            };
            len = i + 1;
        }

        // Find dot position
        let mut dot_pos = None;
        for i in 0..len {
            if name_bytes[i] == b'.' {
                dot_pos = Some(i);
                break;
            }
        }

        if let Some(dot) = dot_pos {
            // Copy base name (up to 8 chars)
            for i in 0..core::cmp::min(dot, 8) {
                fat_name[i] = name_bytes[i];
            }
            // Copy extension (up to 3 chars)
            let ext_start = dot + 1;
            for i in 0..core::cmp::min(len - ext_start, 3) {
                fat_name[8 + i] = name_bytes[ext_start + i];
            }
        } else {
            // No extension, just copy base name
            for i in 0..core::cmp::min(len, 8) {
                fat_name[i] = name_bytes[i];
            }
        }

        self.name == fat_name
    }
}

/// Firmware file handle
///
/// Represents an open firmware file on the SD card with methods to read its contents.
pub struct FirmwareFile {
    /// File size in bytes
    size: u32,
    /// First cluster of the file
    first_cluster: u32,
    /// Current cluster being read
    current_cluster: u32,
    /// Current position within the file
    position: u32,
}

impl FirmwareFile {
    /// Get the file size in bytes
    ///
    /// # Requirements
    /// - 需求 3.3.1: 获取文件大小
    ///
    /// # Returns
    /// File size in bytes
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Get current read position
    pub fn position(&self) -> u32 {
        self.position
    }

    /// Check if end of file reached
    pub fn is_eof(&self) -> bool {
        self.position >= self.size
    }
}

impl<'d, T: Instance, D: SdmmcDma<T>> SdCardReader<'d, T, D> {
    /// Initialize SD card reader with SDIO interface
    ///
    /// Configures the SDIO peripheral with:
    /// - 4-bit data bus mode
    /// - Appropriate clock frequency
    /// - Timeout protection (5 seconds)
    ///
    /// # Pin Configuration
    /// - PC8: SDIO_D0
    /// - PC9: SDIO_D1
    /// - PC10: SDIO_D2
    /// - PC11: SDIO_D3
    /// - PC12: SDIO_CK (Clock)
    /// - PD2: SDIO_CMD (Command)
    ///
    /// # Requirements
    /// - 需求 3.1.1: SDIO 接口初始化
    /// - 需求 3.5.3: 超时保护
    ///
    /// # Arguments
    /// * `sdmmc` - Configured SDMMC peripheral
    ///
    /// # Returns
    /// * `Ok(SdCardReader)` - Successfully initialized SD card reader
    /// * `Err(SdError::InitFailed)` - SD card initialization failed
    /// * `Err(SdError::Timeout)` - Initialization timeout (>5 seconds)
    pub async fn init(mut sdmmc: Sdmmc<'d, T, D>) -> Result<Self, SdError> {
        defmt::info!("Initializing SD card via SDIO interface");

        let timeout = Duration::from_secs(SD_INIT_TIMEOUT.as_secs());

        // Initialize SD card with timeout protection
        let init_result = with_timeout(timeout, async {
            defmt::debug!("Starting SD card initialization");

            sdmmc.init_card(Hertz::khz(400)).await.map_err(|e| {
                defmt::error!("SD card init failed: {:?}", e);
                SdError::InitFailed
            })?;

            defmt::info!("SD card initialized successfully");
            Ok::<(), SdError>(())
        })
        .await;

        match init_result {
            Ok(Ok(())) => {
                defmt::info!("SD card ready (4-bit mode)");
                Ok(Self {
                    sdmmc,
                    fat_info: None,
                })
            }
            Ok(Err(e)) => {
                defmt::error!("SD card initialization error: {:?}", e);
                Err(e)
            }
            Err(_) => {
                defmt::error!("SD card initialization timeout (>5s)");
                Err(SdError::Timeout)
            }
        }
    }

    /// Mount FAT32 filesystem
    ///
    /// Reads the boot sector and initializes FAT32 filesystem information.
    ///
    /// # Requirements
    /// - 需求 3.4.2: 挂载 FAT32 文件系统
    ///
    /// # Returns
    /// * `Ok(())` - Filesystem mounted successfully
    /// * `Err(SdError::MountFailed)` - Failed to mount filesystem
    pub async fn mount(&mut self) -> Result<(), SdError> {
        defmt::info!("Mounting FAT32 filesystem");

        // Read boot sector (sector 0)
        let mut boot_sector = DataBlock([0u8; 512]);
        self.read_block(0, &mut boot_sector).await?;

        // Parse boot sector
        let bytes_per_sector = u16::from_le_bytes([boot_sector.0[11], boot_sector.0[12]]);
        let sectors_per_cluster = boot_sector.0[13];
        let reserved_sectors = u16::from_le_bytes([boot_sector.0[14], boot_sector.0[15]]);
        let num_fats = boot_sector.0[16];
        let fat_size = u32::from_le_bytes([
            boot_sector.0[36],
            boot_sector.0[37],
            boot_sector.0[38],
            boot_sector.0[39],
        ]);
        let root_cluster = u32::from_le_bytes([
            boot_sector.0[44],
            boot_sector.0[45],
            boot_sector.0[46],
            boot_sector.0[47],
        ]);

        // Validate FAT32 signature
        if bytes_per_sector != 512 {
            defmt::error!("Invalid sector size: {}", bytes_per_sector);
            return Err(SdError::MountFailed);
        }

        // Calculate first data sector
        let first_data_sector = reserved_sectors as u32 + (num_fats as u32 * fat_size);

        self.fat_info = Some(Fat32Info {
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            num_fats,
            fat_size,
            root_cluster,
            first_data_sector,
        });

        defmt::info!(
            "FAT32 mounted: {} bytes/sector, {} sectors/cluster, root cluster {}",
            bytes_per_sector,
            sectors_per_cluster,
            root_cluster
        );

        Ok(())
    }

    /// Open firmware file
    ///
    /// Searches for the firmware file in the root directory and returns a file handle.
    ///
    /// # Requirements
    /// - 需求 3.3.1: 查找固件文件
    /// - 需求 3.4.2: 打开固件文件
    ///
    /// # Returns
    /// * `Ok(FirmwareFile)` - File found and opened
    /// * `Err(SdError::FileNotFound)` - Firmware file not found
    /// * `Err(SdError::MountFailed)` - Filesystem not mounted
    pub async fn open_firmware(&mut self) -> Result<FirmwareFile, SdError> {
        defmt::info!("Searching for firmware file: {}", FIRMWARE_FILENAME);

        let fat_info = self.fat_info.ok_or(SdError::MountFailed)?;

        // Read root directory
        let root_sector = fat_info.cluster_to_sector(fat_info.root_cluster);
        let mut sector_buf = DataBlock([0u8; 512]);

        // Search through root directory sectors
        for sector_offset in 0..fat_info.sectors_per_cluster as u32 {
            self.read_block(root_sector + sector_offset, &mut sector_buf)
                .await?;

            // Each sector contains 16 directory entries (512 / 32)
            for entry_idx in 0..16 {
                let offset = entry_idx * 32;
                let entry_bytes = &sector_buf.0[offset..offset + 32];

                // Safety: DirEntry is repr(C, packed) and exactly 32 bytes
                let entry: DirEntry =
                    unsafe { core::ptr::read_unaligned(entry_bytes.as_ptr() as *const DirEntry) };

                if !entry.is_valid() {
                    if entry.name[0] == 0x00 {
                        // End of directory
                        defmt::warn!("Firmware file not found: {}", FIRMWARE_FILENAME);
                        return Err(SdError::FileNotFound);
                    }
                    continue;
                }

                if entry.matches_name(FIRMWARE_FILENAME) {
                    // Copy values to avoid unaligned references
                    let file_size = entry.file_size;
                    let first_cluster = entry.first_cluster();

                    defmt::info!(
                        "Found firmware file: size={} bytes, cluster={}",
                        file_size,
                        first_cluster
                    );

                    return Ok(FirmwareFile {
                        size: file_size,
                        first_cluster,
                        current_cluster: first_cluster,
                        position: 0,
                    });
                }
            }
        }

        defmt::warn!("Firmware file not found after searching root directory");
        Err(SdError::FileNotFound)
    }

    /// Read data from firmware file
    ///
    /// Reads up to `buffer.len()` bytes from the current position in the file.
    ///
    /// # Requirements
    /// - 需求 3.3.1: 读取文件数据
    /// - 需求 3.4.2: 读取数据块
    ///
    /// # Arguments
    /// * `file` - Firmware file handle
    /// * `buffer` - Buffer to read data into
    ///
    /// # Returns
    /// * `Ok(usize)` - Number of bytes read
    /// * `Err(SdError::ReadError)` - Read operation failed
    /// * `Err(SdError::MountFailed)` - Filesystem not mounted
    pub async fn read(
        &mut self,
        file: &mut FirmwareFile,
        buffer: &mut [u8],
    ) -> Result<usize, SdError> {
        if file.is_eof() {
            return Ok(0);
        }

        let fat_info = self.fat_info.ok_or(SdError::MountFailed)?;

        let bytes_to_read = core::cmp::min(buffer.len(), (file.size - file.position) as usize);
        let mut bytes_read = 0;

        while bytes_read < bytes_to_read {
            // Calculate current sector and offset within cluster
            let cluster_offset = file.position % (fat_info.sectors_per_cluster as u32 * 512);
            let sector_in_cluster = cluster_offset / 512;
            let offset_in_sector = (cluster_offset % 512) as usize;

            // Read current sector
            let sector = fat_info.cluster_to_sector(file.current_cluster) + sector_in_cluster;
            let mut sector_buf = DataBlock([0u8; 512]);
            self.read_block(sector, &mut sector_buf).await?;

            // Copy data from sector to buffer
            let bytes_in_sector =
                core::cmp::min(512 - offset_in_sector, bytes_to_read - bytes_read);
            buffer[bytes_read..bytes_read + bytes_in_sector].copy_from_slice(
                &sector_buf.0[offset_in_sector..offset_in_sector + bytes_in_sector],
            );

            bytes_read += bytes_in_sector;
            file.position += bytes_in_sector as u32;

            // Check if we need to move to next cluster
            if file.position % (fat_info.sectors_per_cluster as u32 * 512) == 0
                && bytes_read < bytes_to_read
            {
                // Read FAT to get next cluster
                let fat_sector = fat_info.fat_sector(file.current_cluster);
                let fat_offset = fat_info.fat_offset(file.current_cluster);

                let mut fat_buf = DataBlock([0u8; 512]);
                self.read_block(fat_sector, &mut fat_buf).await?;

                let next_cluster = u32::from_le_bytes([
                    fat_buf.0[fat_offset],
                    fat_buf.0[fat_offset + 1],
                    fat_buf.0[fat_offset + 2],
                    fat_buf.0[fat_offset + 3],
                ]) & 0x0FFFFFFF;

                if next_cluster >= 0x0FFFFFF8 {
                    // End of chain
                    break;
                }

                file.current_cluster = next_cluster;
            }
        }

        Ok(bytes_read)
    }

    /// Get card information
    pub fn card_info(&self) -> Result<CardInfo, SdError> {
        let card = self.sdmmc.card().map_err(|e| {
            defmt::error!("Failed to get card info: {:?}", e);
            SdError::InitFailed
        })?;

        let block_count = card.csd.block_count();
        let capacity_bytes = block_count as u64 * 512u64;
        let capacity_mb = capacity_bytes / (1024 * 1024);

        defmt::info!(
            "SD card info: {} blocks × 512 bytes = {} MB",
            block_count,
            capacity_mb
        );

        Ok(CardInfo {
            block_count,
            block_size: 512,
            capacity_bytes,
        })
    }

    /// Read a single block from SD card
    ///
    /// # Requirements
    /// - 需求 3.4.2: 固件读取
    pub async fn read_block(
        &mut self,
        block_idx: u32,
        buffer: &mut DataBlock,
    ) -> Result<(), SdError> {
        defmt::trace!("Reading block {}", block_idx);

        self.sdmmc
            .read_block(block_idx, buffer)
            .await
            .map_err(|e| {
                defmt::error!("SD card read error at block {}: {:?}", block_idx, e);
                SdError::ReadError
            })?;

        Ok(())
    }

    /// Write a single block to SD card
    pub async fn write_block(&mut self, block_idx: u32, buffer: &DataBlock) -> Result<(), SdError> {
        defmt::trace!("Writing block {}", block_idx);

        self.sdmmc
            .write_block(block_idx, buffer)
            .await
            .map_err(|e| {
                defmt::error!("SD card write error at block {}: {:?}", block_idx, e);
                SdError::ReadError
            })?;

        Ok(())
    }

    /// Read multiple blocks from SD card
    pub async fn read_blocks(
        &mut self,
        start_block: u32,
        blocks: &mut [DataBlock],
    ) -> Result<(), SdError> {
        for (i, block) in blocks.iter_mut().enumerate() {
            self.read_block(start_block + i as u32, block).await?;
        }
        Ok(())
    }

    /// Write multiple blocks to SD card
    pub async fn write_blocks(
        &mut self,
        start_block: u32,
        blocks: &[DataBlock],
    ) -> Result<(), SdError> {
        for (i, block) in blocks.iter().enumerate() {
            self.write_block(start_block + i as u32, block).await?;
        }
        Ok(())
    }
}

/// SD card information
#[derive(Debug, Clone, Copy)]
pub struct CardInfo {
    /// Total number of blocks on the card
    pub block_count: u32,
    /// Size of each block in bytes (always 512)
    pub block_size: u32,
    /// Total capacity in bytes
    pub capacity_bytes: u64,
}

impl CardInfo {
    /// Get capacity in megabytes
    pub fn capacity_mb(&self) -> u64 {
        self.capacity_bytes / (1024 * 1024)
    }

    /// Get capacity in gigabytes
    pub fn capacity_gb(&self) -> f32 {
        self.capacity_bytes as f32 / (1024.0 * 1024.0 * 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_info_capacity() {
        let info = CardInfo {
            block_count: 1024 * 1024 * 2,
            block_size: 512,
            capacity_bytes: 1024 * 1024 * 1024,
        };

        assert_eq!(info.capacity_mb(), 1024);
        assert!((info.capacity_gb() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_dir_entry_name_matching() {
        let mut entry = DirEntry {
            name: *b"FIRMWAREBIN",
            attr: 0x20, // Archive
            nt_reserved: 0,
            create_time_tenth: 0,
            create_time: 0,
            create_date: 0,
            last_access_date: 0,
            first_cluster_hi: 0,
            write_time: 0,
            write_date: 0,
            first_cluster_lo: 0,
            file_size: 0,
        };

        assert!(entry.matches_name("firmware.bin"));
        assert!(entry.matches_name("FIRMWARE.BIN"));
        assert!(!entry.matches_name("other.bin"));
    }

    #[test]
    fn test_firmware_file_eof() {
        let file = FirmwareFile {
            size: 1024,
            first_cluster: 2,
            current_cluster: 2,
            position: 0,
        };

        assert!(!file.is_eof());
        assert_eq!(file.size(), 1024);

        let mut file_at_end = file;
        file_at_end.position = 1024;
        assert!(file_at_end.is_eof());
    }
}
