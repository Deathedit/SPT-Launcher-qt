//! Live check of the sp-mod.com client (read-only, installs into a temp dir):
//!   cargo run -p spt-core --example forge_probe -- <mod id or url> [spt version]
use spt_core::{forge, mods};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let id = forge::parse_mod_ref(args.get(1).map(String::as_str).unwrap_or("1775")).expect("mod id or sp-mod.com url");
    let spt = args.get(2).cloned().unwrap_or_else(|| "4.1.6".into());
    let f = forge::Forge::new();

    let page = f.search("menu overhaul", &spt, 1)?;
    println!("search: {} results, first: {:?}", page.mods.len(), page.mods.first().map(|m| (&m.name, m.author())));

    let m = f.get_mod(id)?;
    println!("mod: {} by {} ({})", m.name, m.author(), m.page_url());
    let v = f.latest_version(id, &spt)?.expect("no compatible version");
    println!("version: {} for SPT {} · {} bytes · vt={}", v.version, v.spt_version_constraint, v.content_length, v.virus_total_links.len());
    for d in f.dependencies(id, &v.version, &spt)? {
        println!("dependency: {} {:?}", d.name, d.latest_compatible_version.map(|v| v.version));
    }

    let dir = std::env::temp_dir().join(format!("forge-probe-{id}"));
    let _ = std::fs::remove_dir_all(&dir);
    let name = f.download(&v.link, &dir.join("archive"), |_, _| {})?;
    let archive = dir.join(if name.is_empty() { "archive.zip".into() } else { name.clone() });
    std::fs::rename(dir.join("archive"), &archive)?;
    println!("downloaded: {name} ({} bytes)", std::fs::metadata(&archive)?.len());
    mods::extract(&archive, &dir.join("x"))?;
    let plan = mods::plan(&dir.join("x"))?;
    for file in &plan.files {
        println!("  {:?} {}", file.part, file.dest);
    }
    println!("ignored: {:?}", plan.ignored);
    let root = dir.join("spt");
    let files = mods::apply(&root, &plan, false)?;
    println!("installed {} files into {}", files.len(), root.display());
    Ok(())
}
