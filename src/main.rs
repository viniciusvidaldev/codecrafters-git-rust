use std::{
    ffi::CStr,
    fs,
    io::{self, BufRead, BufReader, Read},
};

use anyhow::Context;
use clap::{Parser, Subcommand};
use flate2::read::ZlibDecoder;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

/// Doc comment
#[derive(Debug, Subcommand)]
enum Command {
    /// Doc comment
    Init,
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,

        object_hash: String,
    },
}

enum Kind {
    Blob,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // You can use print statements as follows for debugging, they'll be visible when running tests.
    eprintln!("Logs from your program will appear here!");

    match args.command {
        Command::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory")
        }
        Command::CatFile {
            pretty_print,
            object_hash,
        } => {
            anyhow::ensure!(
                pretty_print,
                "mode must be given without -p, and we don't support mode"
            );

            // TODO: support shortest-unique object hashes
            let f = std::fs::File::open(format!(
                ".git/objects/{}/{}",
                &object_hash[..2],
                &object_hash[2..],
            ))
            .context("open in .git/objects")?;
            let decoder = ZlibDecoder::new(f);
            let mut reader = BufReader::new(decoder);
            let mut buf = Vec::new();
            reader
                .read_until(0, &mut buf)
                .context("read header from .git/objects")?;

            let header = CStr::from_bytes_until_nul(&buf)
                .expect("know there is exactly one nul, and it's at the end");
            let header = header
                .to_str()
                .context(".git/objects file header isn't valid UTF-8")?;

            let Some((kind, size)) = header.split_once(' ') else {
                anyhow::bail!(".git/objects file header did not start with know type: '{header}'");
            };

            let kind = match kind {
                "blob" => Kind::Blob,
                _ => anyhow::bail!("we do not yet know how to print a '{kind}'"),
            };

            let size = size
                .parse::<u64>()
                .context(".git/objects file header has invalid size: '{size}'")?;

            // NOTE: this won't error if the decompressed file is too long, but it will at least
            // not spam stdout and be vunerable to a zipbomb
            let mut reader = reader.take(size);
            match kind {
                Kind::Blob => {
                    let stdout = std::io::stdout();
                    let mut stdout = stdout.lock();
                    let copied_bytes_n = std::io::copy(&mut reader, &mut stdout)
                        .context("write .git/objects to stdout")?;

                    anyhow::ensure!(
                        copied_bytes_n == size,
                        ".git/object file was not the expected size (expected: {size}, actual: {copied_bytes_n})"
                    );
                }
            }
        }
    }

    Ok(())
}
