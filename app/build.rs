use git2::{DescribeOptions, Repository};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut opts = DescribeOptions::new();
    opts.describe_tags().show_commit_oid_as_fallback(true);
    let version = Repository::discover(".")?.describe(&opts)?.format(None)?;
    println!("cargo:rustc-env=LCP_VERSION={}", version);
    Ok(())
}
