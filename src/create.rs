use anyhow::Result;
use clap::{Parser, ValueHint};
use std::cell::Cell;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Parser)]
pub struct Options {
    // Archive name to create
    #[arg(short = 'f', long = "file", value_parser)]
    outfile: PathBuf,

    /// Remove STRIP_PREFIX from the entries' name added to the archive.
    #[arg(long, required = false, value_hint=ValueHint::DirPath)]
    strip_prefix: Option<PathBuf>,

    /// Move to BASE_DIR before starting adding content to arx archive.
    ///
    /// Argument `INFILES` or `STRIP_PREFIX` must be relative to `BASE_DIR`.
    #[arg(short = 'C', required = false, value_hint=ValueHint::DirPath)]
    base_dir: Option<PathBuf>,

    /// Input files/directories
    ///
    /// This is an option incompatible with `FILE_LIST`.
    #[arg(value_parser, group = "input", value_hint=ValueHint::AnyPath)]
    infiles: Vec<PathBuf>,

    /// Get the list of files/directories to add from the FILE_LIST (incompatible with INFILES)
    ///
    /// This is an option incompatible with `INFILES`.
    #[arg(short = 'L', long = "file-list", group = "input", verbatim_doc_comment, value_hint=ValueHint::FilePath)]
    file_list: Option<PathBuf>,

    #[arg(short = '1', long, required = false, default_value_t = false, action)]
    one_file: bool,

    #[arg(short, long, required = false)]
    main: Option<String>,

    #[arg(from_global)]
    verbose: u8,
}

fn get_files_to_add(options: &Options) -> jbk::Result<Vec<PathBuf>> {
    if let Some(file_list) = &options.file_list {
        let file = File::open(file_list)?;
        let mut files = Vec::new();
        for line in BufReader::new(file).lines() {
            files.push(line?.into());
        }
        Ok(files)
    } else {
        Ok(options.infiles.clone())
    }
}

struct ProgressBar {
    comp_clusters: indicatif::ProgressBar,
    uncomp_clusters: indicatif::ProgressBar,
}

impl ProgressBar {
    fn new() -> Self {
        let style = indicatif::ProgressStyle::with_template(
            "{prefix} : {wide_bar:.cyan/blue} {pos:4} / {len:4}",
        )
        .unwrap()
        .progress_chars("#+-");
        let multi = indicatif::MultiProgress::new();
        let comp_clusters = indicatif::ProgressBar::new(0)
            .with_style(style.clone())
            .with_prefix("Compressed Cluster  ");
        let uncomp_clusters = indicatif::ProgressBar::new(0)
            .with_style(style)
            .with_prefix("Uncompressed Cluster");
        multi.add(comp_clusters.clone());
        multi.add(uncomp_clusters.clone());
        Self {
            comp_clusters,
            uncomp_clusters,
        }
    }
}

impl jbk::creator::Progress for ProgressBar {
    fn new_cluster(&self, _cluster_idx: u32, compressed: bool) {
        if compressed {
            &self.comp_clusters
        } else {
            &self.uncomp_clusters
        }
        .inc_length(1)
    }
    fn handle_cluster(&self, _cluster_idx: u32, compressed: bool) {
        if compressed {
            &self.comp_clusters
        } else {
            &self.uncomp_clusters
        }
        .inc(1)
    }
}

struct CachedSize(Cell<u64>);

impl jbk::creator::CacheProgress for CachedSize {
    fn cached_data(&self, size: jbk::Size) {
        self.0.set(self.0.get() + size.into_u64());
    }
}

impl CachedSize {
    fn new() -> Self {
        Self(Cell::new(0))
    }
}

pub fn create(options: Options) -> Result<()> {
    if options.verbose > 0 {
        println!("Creating archive {:?}", options.outfile);
        println!("With files {:?}", options.infiles);
    }

    let strip_prefix = match &options.strip_prefix {
        Some(s) => s.clone(),
        None => PathBuf::new(),
    };

    let out_file = std::env::current_dir()?.join(&options.outfile);

    let concat_mode = if options.one_file {
        waj::create::ConcatMode::OneFile
    } else {
        waj::create::ConcatMode::TwoFiles
    };

    let jbk_progress = Arc::new(ProgressBar::new());
    let progress = Rc::new(CachedSize::new());
    let mut creator = waj::create::FsCreator::new(
        &out_file,
        strip_prefix,
        concat_mode,
        jbk_progress,
        Rc::clone(&progress) as Rc<dyn jbk::creator::CacheProgress>,
    )?;

    let files_to_add = get_files_to_add(&options)?;

    if let Some(base_dir) = &options.base_dir {
        std::env::set_current_dir(base_dir)?;
    };

    for infile in files_to_add {
        creator.add_from_path(&infile)?;
    }

    if let Some(main_page) = options.main {
        creator.add_redirect("", &main_page)?;
    }

    let ret = creator.finalize(&out_file);
    Ok(ret?)
}
