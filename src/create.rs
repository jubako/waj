use anyhow::{anyhow, Context, Result};
use clap::{Parser, ValueHint};
use std::cell::Cell;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use waj::create::StripPrefix;

#[derive(Parser)]
pub struct Options {
    /// File path of the archive to create.
    ///
    /// Relative path are relative to the current working dir.
    /// `BASE_DIR` option is used after resolving relative path.
    #[arg(
        short,
        long,
        value_parser,
        required_unless_present("list_compressions"),
        value_hint=ValueHint::FilePath,
    )]
    outfile: Option<PathBuf>,

    /// Remove STRIP_PREFIX from the entries' name added to the archive.
    #[arg(long, required = false, value_hint=ValueHint::DirPath)]
    strip_prefix: Option<PathBuf>,

    /// Move to BASE_DIR before starting adding content to arx archive.
    ///
    /// Argument `INFILES` or `STRIP_PREFIX` must be relative to `BASE_DIR`.
    /// `OUTFILE` and `FILE_LIST` path is always relative to currrent directory.
    /// Paths listed in `FILE_LIST` are related to `BASE_DIR`
    #[arg(short = 'C', required = false, value_hint=ValueHint::DirPath)]
    base_dir: Option<PathBuf>,

    /// Input files/directories
    ///
    /// This is an option incompatible with `FILE_LIST`.
    #[arg(group = "input", value_hint=ValueHint::AnyPath)]
    infiles: Vec<PathBuf>,

    /// Get the list of files/directories to add from the FILE_LIST (incompatible with INFILES)
    ///
    /// This is an option incompatible with `INFILES`.
    ///
    /// Relative path are relative to the current working dir. `BASE_DIR` option is used after resolving relative path.
    #[arg(short = 'L', long = "file-list", group = "input", verbatim_doc_comment, value_hint=ValueHint::FilePath)]
    file_list: Option<PathBuf>,

    #[command(flatten)]
    concat_mode: Option<jbk::cmd_utils::ConcatMode>,

    /// Set compression algorithm to use
    #[arg(short, long, value_parser=jbk::cmd_utils::compression_arg_parser, required=false, default_value="zstd")]
    compression: jbk::creator::Compression,

    /// List available compression algorithms
    #[arg(long, default_value_t = false, action)]
    list_compressions: bool,

    #[arg(short, long, required = false, default_value_t = false, action)]
    force: bool,

    #[arg(short, long, required = false)]
    main: Option<String>,

    #[arg(from_global)]
    verbose: u8,
}

fn check_output_path_writable(out_file: &Path, force: bool) -> Result<()> {
    if !out_file.parent().unwrap().is_dir() {
        Err(anyhow!(
            "Directory {} doesn't exist",
            out_file.parent().unwrap().display()
        ))
    } else if out_file.exists() && !force {
        Err(anyhow!(
            "File {} already exists. Use option --force to overwrite it.",
            out_file.display()
        ))
    } else {
        Ok(())
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
    if options.list_compressions {
        jbk::cmd_utils::list_compressions();
        return Ok(());
    }

    if options.verbose > 0 {
        println!("Creating archive {:?}", options.outfile);
        println!("With files {:?}", options.infiles);
    }

    let strip_prefix = match &options.strip_prefix {
        Some(s) => s.clone(),
        None => PathBuf::new(),
    };

    let out_file = options.outfile.as_ref().expect(
        "Clap unsure it is Some, except if we have list_compressions, and so we return early",
    );
    let out_file = std::env::current_dir()?.join(out_file);
    check_output_path_writable(&out_file, options.force)?;

    let file_list = options
        .file_list
        .as_ref()
        .map(std::path::absolute)
        .transpose()?;

    if let Some(base_dir) = &options.base_dir {
        std::env::set_current_dir(base_dir)?;
    };

    let jbk_progress = Arc::new(ProgressBar::new());
    let progress = Rc::new(CachedSize::new());

    let namer = Box::new(StripPrefix::new(strip_prefix));
    let mut creator = waj::create::FsCreator::new(
        &out_file,
        namer,
        match options.concat_mode {
            None => jbk::creator::ConcatMode::OneFile,
            Some(e) => e.into(),
        },
        jbk_progress,
        Rc::clone(&progress) as Rc<dyn jbk::creator::CacheProgress>,
        options.compression,
    )?;

    let files_to_add = if let Some(file_list) = file_list {
        let file = File::open(&file_list)
            .with_context(|| format!("Cannot open {}", file_list.display()))?;
        BufReader::new(file)
            .lines()
            .map(|l| -> Result<PathBuf> { Ok(l?.into()) })
            .collect::<Result<Vec<_>>>()?
    } else {
        options
            .infiles
            .iter()
            .map(|f| -> Result<PathBuf> {
                if f.is_absolute() {
                    Err(anyhow!("Input file ({}) must be relative.", f.display()))
                } else {
                    Ok(f.clone())
                }
            })
            .collect::<Result<Vec<_>>>()?
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
