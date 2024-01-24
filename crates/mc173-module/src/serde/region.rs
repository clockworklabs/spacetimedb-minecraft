//! Minecraft region file format storing 32x32 chunks inside a single file.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::{self, Seek, SeekFrom, Write, Read};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Take;

use byteorder::{ReadBytesExt, WriteBytesExt};

use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::io::{ReadJavaExt, WriteJavaExt};


/// Internal function to calculate the index of a chunk metadata depending on its 
/// position, this is the same calculation as Notchian server.
#[inline]
fn calc_chunk_meta_index(cx: i32, cz: i32) -> usize {
    (cx & 31) as usize | (((cz & 31) as usize) << 5)
}

/// Internal constant empty array of 4K to write an empty sector.
const EMPTY_SECTOR: &'static [u8; 4096] = &[0; 4096];

/// A handle to a region directory storing all region files.
pub struct RegionDir {
    /// Path of the region directory.
    path: PathBuf,
    /// Cache of already loaded region files.
    cache: HashMap<(i32, i32), Region<File>>,
}

impl RegionDir {
    
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            cache: HashMap::new(),
        }
    }

    /// Ensure that a region file exists for the given chunk coordinates. If false is
    /// given as the 'create' argument, then the file will not be created and initialized
    /// if not existing.
    pub fn ensure_region(&mut self, cx: i32, cz: i32, create: bool) -> Result<&mut Region<File>, RegionError> {
        let (rx, rz) = (cx >> 5, cz >> 5);
        match self.cache.entry((rx, rz)) {
            Entry::Occupied(o) => Ok(o.into_mut()),
            Entry::Vacant(v) => {
                Ok(v.insert(Region::open(self.path.join(format!("r.{rx}.{rz}.mcr")), create)?))
            }
        }
    }

}

/// A handle to a region file. This is an implementation of ".mcr" region files following
/// the same algorithms as the Notchian server, first developed by Scaevolus (legend!).
/// 
/// Being generic over `I` allows us to use a mockup inner for tests.
pub struct Region<I> {
    /// Underlying read/writer with seek. 
    inner: I,
    /// Stores the metadata of each chunks
    chunks: Box<[ChunkMeta; 1024]>,
    /// Bit mapping of sectors that are allocated.
    sectors: Vec<u64>,
}

impl Region<File> {

    /// Open a region file, this constructor report every possible error with the region
    /// file without altering it in such case, it's up to the caller to delete the file
    /// and retry is wanted.
    pub fn open<P: AsRef<Path>>(path: P, create: bool) -> Result<Self, RegionError> {

        let path: &Path = path.as_ref();

        if create {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let file = File::options()
            .read(true)
            .write(true)
            .create(create)
            .open(path)?;

        Self::new(file, create)
        
    }

}

impl<I> Region<I>
where
    I: Read + Write + Seek,
{

    /// Create a new region around a inner reader/writer with seek. This constructor
    /// also reads initial data and also check for file integrity.
    pub fn new(mut inner: I, create: bool) -> Result<Self, RegionError> {

        // Start by querying the file length.
        let mut file_len = inner.seek(SeekFrom::End(0))?;

        // A region file should have a length of at least 8K in order to store each chunk
        // metadata, we fix the file if this is not already the case.
        if file_len == 0 && create {
            // We write all first 8K to zero to initialize the file, only if it was meant
            // to be created, if not then we fall through and return too small error.
            for _ in 0..2 {
                inner.write_all(EMPTY_SECTOR)?;
            }
            file_len = 8192;
        } else if file_len < 8192 {
            return Err(RegionError::FileTooSmall(file_len));
        } else if file_len % 4096 != 0 {
            return Err(RegionError::FileNotPadded(file_len));
        }

        // The first two sectors are always reserved for the chunk metadata.
        let mut chunks: Box<[ChunkMeta; 1024]> = Box::new([ChunkMeta::INIT; 1024]);
        let mut sectors = vec![0u64; file_len as usize / 4096];
        // First two sectors are reserved for headers.
        sectors[0] |= 0b11;

        inner.seek(SeekFrom::Start(0))?;

        // Start by reading each offset, 4 bytes each for 1024 chunks, so 4K.
        for i in 0..1024 {

            let range_raw = inner.read_java_int()? as u32;
            let range = SectorRange {
                offset: range_raw >> 8,
                count: range_raw & 0xFF,
            };

            chunks[i].range = range;

            for offset in range.offset..range.offset + range.count {
                if let Some(slot) = sectors.get_mut(offset as usize / 64) {
                    *slot |= 1u64 << (offset % 64);
                } else {
                    return Err(RegionError::IllegalRange);
                }
            }

        }

        // Then we read the timestamps, same format as offsets.
        for i in 0..1024 {
            chunks[i].timestamp = inner.read_java_int()? as u32;
        }

        Ok(Self {
            inner,
            chunks,
            sectors,
        })

    }

    /// Internal function to get the chunk metadata associated with a chunk.
    fn get_chunk_meta(&self, cx: i32, cz: i32) -> ChunkMeta {
        self.chunks[calc_chunk_meta_index(cx, cz)]
    }

    /// Internal function to set the chunk metadata and synchronize 
    fn set_chunk_meta_and_sync(&mut self, cx: i32, cz: i32, chunk: ChunkMeta) -> io::Result<()> {
        let index = calc_chunk_meta_index(cx, cz);
        // Synchronize range.
        let range_raw = chunk.range.offset << 8 | chunk.range.count & 0xFF;
        let header_offset = index as u64 * 4;
        self.inner.seek(SeekFrom::Start(header_offset))?;
        self.inner.write_java_int(range_raw as i32)?;
        // Finally write the inner metadata because we are fully synchronized.
        // NOTE: We don't really care of the timestamp, we critically need to have the
        // range synchronized, the timestamp is less important.
        self.chunks[index] = chunk;
        // Synchronize timestamp.
        self.inner.seek(SeekFrom::Start(header_offset + 4096))?;
        self.inner.write_java_int(chunk.timestamp as i32)?;
        Ok(())
    }

    /// Read the chunk at the given position, the chunk position is at modulo 32 in order
    /// to respect the limitations of the region size, caller don't have to do it.
    pub fn read_chunk(&mut self, cx: i32, cz: i32) -> Result<ChunkReader<'_, I>, RegionError> {

        let chunk = self.get_chunk_meta(cx, cz);
        if chunk.is_empty() {
            return Err(RegionError::EmptyChunk);
        }

        if chunk.range.offset < 2 {
            return Err(RegionError::IllegalRange);
        }

        // Seek to the start of the chunk where the header is present.
        self.inner.seek(SeekFrom::Start(chunk.range.offset as u64 * 4096))?;

        let chunk_size = self.inner.read_java_int()?;
        if chunk_size <= 0 || chunk_size as u32 + 4 > chunk.range.count * 4096 {
            return Err(RegionError::IllegalRange);
        }

        let compression_id = self.inner.read_u8()?;
        let chunk_size = chunk_size as u64 - 1; // Subtract one for compression id.
        let chunk_data = Read::take(&mut self.inner, chunk_size);

        // println!("reading chunk {cx}/{cz}, offset = {}, count = {}, size = {}, compression = {}", chunk.range.offset, chunk.range.count, chunk_size, compression_id);

        let inner = match compression_id {
            1 => ChunkReaderInner::Gz(GzDecoder::new(chunk_data)),
            2 => ChunkReaderInner::Zlib(ZlibDecoder::new(chunk_data)),
            _ => return Err(RegionError::IllegalCompression),
        };

        Ok(ChunkReader { inner })

    }

    /// Write a chunk at the given position, the chunk position is at modulo 32 in order
    /// to respect the limitations of the region size, caller don't have to do it.
    pub fn write_chunk(&mut self, cx: i32, cz: i32) -> ChunkWriter<'_, I> {
        ChunkWriter {
            cx, 
            cz, 
            encoder: ZlibEncoder::new(Vec::new(), Compression::best()), 
            region: self,
        }
    }

    fn write_chunk_data(&mut self, cx: i32, cz: i32, compression_id: u8, data: &[u8]) -> Result<(), RegionError> {
 
        // NOTE: This will always require at least 1 sector because of headers.
        let sector_count = (data.len() + 5 - 1) as u32 / 4096 + 1;
        if sector_count > 0xFF {
            return Err(RegionError::OutOfSector);
        }

        let mut chunk = self.get_chunk_meta(cx, cz);

        // This assert is critical to ensure that no data corruption happens if we made
        // any programming mistakes.
        assert!(chunk.range.count == 0 || chunk.range.offset >= 2, "previous chunk metadata uses reserved sectors");
        // Just a sanity check.
        debug_assert!(sector_count != 0);

        // If the current chunk count doesn't match, we try to extend the current one or
        // allocate a new available range. This can also happen if the old chunk metadata
        // was zero in length.
        if sector_count != chunk.range.count {

            let mut clear_range = chunk.range;

            // We just need to shrink sectors. If count was zero then nothing is cleared
            // and this cause no problem.
            if sector_count < chunk.range.count {
                clear_range.offset += sector_count;
                clear_range.count -= sector_count;
                chunk.range.count = sector_count;
            }

            // Clear the previous range.
            if clear_range.count != 0 {
                self.inner.seek(SeekFrom::Start(clear_range.offset as u64 * 4096))?;
                for offset in clear_range.offset..clear_range.offset + clear_range.count {
                    let slot = &mut self.sectors[offset as usize / 64];
                    *slot &= !(1u64 << (offset % 64));
                    self.inner.write_all(EMPTY_SECTOR)?;
                }
            }

            // If we did not shrink, we have deallocated everything so we need to alloc.
            if sector_count > chunk.range.count {

                // Debug assert to ensure that our internal sectors always keep first two
                // sectors as reserved, this is just a _debug_ assert because we later 
                // ensures that no data is written in these sectors.
                debug_assert_eq!(self.sectors[0] & 0b11, 0b11);

                let mut new_range = SectorRange::default();

                // NOTE: Our sectors should always be filled 
                'out: for (slot_index, mut slot) in self.sectors.iter().copied().enumerate() {
                    // Avoid check a slot that is fully allocated.
                    if slot != u64::MAX {
                        // Check for each slot bit for a sequence of free sectors.
                        for bit_index in 0usize..64 {
                            if slot & 1 == 0 {
                                new_range.count += 1;
                                if new_range.count == sector_count {
                                    break 'out;
                                }
                            } else {
                                new_range.offset = slot_index as u32 * 64 + bit_index as u32 + 1;
                                new_range.count = 0;
                            }
                            slot >>= 1;
                        }
                    } else {
                        // If the slot is full, we still have to update the range...
                        new_range.offset = slot_index as u32 * 64 + 64;
                        new_range.count = 0;
                    }
                }

                // Check to avoid corruption of the internal sectors mapping.
                assert!(new_range.offset >= 2, "allocating reserved sectors");

                // NOTE: We are overwriting the count because if we did not find enough
                // free space we can still add it at the end.
                new_range.count = sector_count;
                for offset in new_range.offset..new_range.offset + new_range.count {
                    let slot_index = offset as usize / 64;
                    if let Some(slot) = self.sectors.get_mut(slot_index) {
                        *slot |= 1u64 << (offset % 64);
                    } else {
                        debug_assert_eq!(slot_index, self.sectors.len());
                        self.sectors.push(1u64 << (offset % 64));
                    }
                }

                chunk.range = new_range;

            }

        }

        // This assert is critical if we failed our implementation to avoid corruption.
        assert!(chunk.range.offset >= 2, "allocating reserved sectors");
        // Because of the header, we should always end with at least one sector.
        assert!(chunk.range.count != 0, "allocating zero sector");

        // Write the metadata and then write the chunk data.
        self.set_chunk_meta_and_sync(cx, cz, chunk)?;
        self.inner.seek(SeekFrom::Start(chunk.range.offset as u64 * 4096))?;
        self.inner.write_java_int(data.len() as i32 + 1)?; // Counting the compression id.
        self.inner.write_u8(compression_id)?;
        self.inner.write_all(data)?;

        // Calculate the zero padding to write at the end in order to have a file that
        // is 4K aligned, and also to clear old data.
        let total_len = data.len() + 4 + 1;
        let padding_len = 4096 - total_len % 4096;
        self.inner.write_all(&EMPTY_SECTOR[..padding_len])?;
        self.inner.flush()?;

        Ok(())

    }

}


/// A handle for reading a chunk from a region file.
pub struct ChunkReader<'region, I> {
    /// Inner implementation depending on compression.
    inner: ChunkReaderInner<'region, I>,
}

/// The actual implementation of the chunk reader depending on the compression type.
enum ChunkReaderInner<'region, I> {
    Gz(GzDecoder<Take<&'region mut I>>),
    Zlib(ZlibDecoder<Take<&'region mut I>>),
}

impl<I> Read for ChunkReader<'_, I>
where
    I: Read + Write + Seek,
{

    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match &mut self.inner {
            ChunkReaderInner::Gz(gz) => gz.read(buf),
            ChunkReaderInner::Zlib(zlib) => zlib.read(buf),
        }
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        match &mut self.inner {
            ChunkReaderInner::Gz(gz) => gz.read_exact(buf),
            ChunkReaderInner::Zlib(zlib) => zlib.read_exact(buf),
        }
    }

}


/// A handle for writing a chunk in a region file.
pub struct ChunkWriter<'region, I> {
    /// The chunk X coordinate.
    cx: i32,
    /// The chunk Z coordinate.
    cz: i32,
    /// The internal zlib encoder, we force using zlib when writing (id 2).
    encoder: ZlibEncoder<Vec<u8>>,
    /// The underlying region file used to finally write chunk data.
    region: &'region mut Region<I>,
}

impl<I> ChunkWriter<'_, I> 
where
    I: Read + Write + Seek,
{

    /// A more costly variant of the regular IO's flush function, because this one also
    /// flush the inner encoded buffer to the region file, therefore searching available
    /// sectors and writing data.
    pub fn flush_chunk(self) -> Result<(), RegionError> {
        let inner = self.encoder.flush_finish()?;
        self.region.write_chunk_data(self.cx, self.cz, 2, &inner)
    }

}

impl<I> Write for ChunkWriter<'_, I> {

    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.encoder.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        // We are not flushing here because it's only relevant to flush in `flush_chunk`.
        Ok(())
    }

}


/// Internal cached chunk metadata, it is kept in-sync with the region file.
#[derive(Debug, Clone, Copy)]
struct ChunkMeta {
    /// The offset of the chunk in sectors within the region file. The least significant
    /// byte is used for counting the number of sectors used (can be zero), and the 
    /// remaining 3 bytes are storing the offset in sectors (should not be 0 or 1).
    range: SectorRange,
    /// Timestamp when the chunk was last saved in the region file.
    timestamp: u32,
}

impl ChunkMeta {

    const INIT: Self = Self { range: SectorRange { offset: 0, count: 0 }, timestamp: 0 };

    fn is_empty(self) -> bool {
        self.range.is_empty()
    }

}

/// Indicate a free range of sector.
#[derive(Debug, Clone, Copy, Default)]
struct SectorRange {
    /// Offset of the first sector in that range.
    offset: u32,
    /// The number of sectors in the ranges, this should not be zero.
    count: u32,
}

impl SectorRange {

    fn is_empty(self) -> bool {
        self.count == 0
    }

}

/// Error type used together with `RegionResult` for every call on region file methods.
#[derive(thiserror::Error, Debug)]
pub enum RegionError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("the region file size ({0}) is too short to store the two 4K header sectors")]
    FileTooSmall(u64),
    #[error("the region file size ({0}) is not a multiple of 4K")]
    FileNotPadded(u64),
    #[error("the region file has an invalid chunk range, likely out of range or colliding with another one")]
    IllegalRange,
    #[error("the required chunk is empty, it has no sector allocated in the region file")]
    EmptyChunk,
    #[error("the compression method in the chunk header is illegal")]
    IllegalCompression,
    #[error("no more sectors are available in the region file, really unlikely to happen")]
    OutOfSector,
}
