use memmap2::MmapMut;
use std::collections::HashMap;
use std::io;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{SystemTime, UNIX_EPOCH};

fn increment() -> u64 {
    4 * page_size::get() as u64
}

struct U64ColWriter {
    file: std::fs::File,
    mmap: MmapMut,
    len: u64,
    cap: u64,
}

impl U64ColWriter {
    fn new(path: &Path) -> io::Result<Self> {
        let cap = increment();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .unwrap();
        file.set_len(cap)?;
        let mmap = unsafe { MmapMut::map_mut(&file)? };
        Ok(Self {
            file,
            mmap,
            len: 0,
            cap,
        })
    }

    fn may_resize(&mut self) -> io::Result<()> {
        let required = self.len + size_of::<u64>() as u64;
        if required < self.cap {
            return Ok(());
        }
        self.cap += increment();
        self.file.set_len(self.cap)?;
        self.mmap = unsafe { MmapMut::map_mut(&self.file)? };
        Ok(())
    }

    fn append(&mut self, val: u64) -> io::Result<()> {
        self.may_resize()?;
        let offset = self.len as usize;
        self.mmap[offset..offset + size_of::<u64>()].copy_from_slice(&val.to_le_bytes());
        self.len += size_of::<u64>() as u64;
        Ok(())
    }
}

struct CountWriter {
    mmap: MmapMut,
}

impl CountWriter {
    fn new(path: &Path) -> io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .unwrap();
        file.set_len(size_of::<u64>() as u64)?;
        let mmap = unsafe { MmapMut::map_mut(&file)? };
        let mut count_writer = Self { mmap };
        count_writer.set(0)?;
        Ok(count_writer)
    }

    fn get(&self) -> u64 {
        u64::from_le_bytes(self.mmap[0..size_of::<u64>()].try_into().unwrap())
    }

    fn set(&mut self, val: u64) -> io::Result<()> {
        self.mmap[0..size_of::<u64>()].copy_from_slice(&val.to_le_bytes());
        Ok(())
    }

    fn increment(&mut self) -> io::Result<()> {
        let count = self.get();
        self.set(count + 1)
    }
}

struct DatapointWriter {
    ts_writer: U64ColWriter,
    val_writer: U64ColWriter,
    count_writer: CountWriter,
}

impl DatapointWriter {
    fn new(path: &Path) -> io::Result<Self> {
        let ts_writer = U64ColWriter::new(&path.with_extension("ts"))?;
        let val_writer = U64ColWriter::new(&path.with_extension("val"))?;
        let count_writer = CountWriter::new(&path.with_extension("count"))?;
        Ok(Self {
            ts_writer,
            val_writer,
            count_writer,
        })
    }

    fn append(&mut self, ts: u64, val: u64) -> io::Result<()> {
        self.ts_writer.append(ts)?;
        self.val_writer.append(val)?;
        self.count_writer.increment()
    }
}

pub struct Writer {
    root_dir: PathBuf,
    datapoint_writers: HashMap<u16, DatapointWriter>,
}

impl Writer {
    fn new(root_dir: PathBuf) -> io::Result<Self> {
        if root_dir.exists() {
            std::fs::remove_dir_all(&root_dir)?;
        }
        std::fs::create_dir_all(&root_dir)?;
        Ok(Self {
            root_dir,
            datapoint_writers: HashMap::new(),
        })
    }

    fn append(&mut self, port: u16, ts: SystemTime, val: u64) -> io::Result<()> {
        let epoch_nanos = ts.duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
        if let Some(writer) = self.datapoint_writers.get_mut(&port) {
            writer.append(epoch_nanos, val)
        } else {
            let path = self.root_dir.join(format!("{}", port));
            let mut writer = DatapointWriter::new(&path)?;
            writer.append(epoch_nanos, val)?;
            self.datapoint_writers.insert(port, writer);
            Ok(())
        }
    }

    pub fn run(dir: PathBuf) -> Sender<Record> {
        use std::sync::mpsc;
        use std::thread;
        let (tx, rx) = mpsc::channel::<Record>();
        thread::spawn(move || {
            let mut writer = Writer::new(dir).unwrap();
            for record in rx {
                writer.append(record.port, record.ts, record.val).unwrap();
            }
        });
        tx
    }
}

pub struct Record {
    pub(crate) port: u16,
    pub(crate) ts: SystemTime,
    pub(crate) val: u64,
}
