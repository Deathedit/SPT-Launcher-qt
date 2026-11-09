//! Read-only diagnostics: `cargo run -p spt-core --example probe [host]`
use spt_core::{api::Client, config::*, game, proton, steam};

fn main() {
    let conf = AppConf::load();
    let install = SptInstall::new(resolve_spt_path(&conf));
    println!("spt path:     {} (installed: {})", install.root.display(), install.is_installed());
    println!("server mode:  {}", conf.server_enabled());
    match game::pe_file_version(&install.core_dll()) {
        Ok(v) => println!("spt-core.dll: {}.{}.{}", v.0, v.1, v.2),
        Err(e) => println!("spt-core.dll: {e}"),
    }
    let s = LauncherSettings::load(&install);
    println!("prefix:       {}", s.prefix_path());
    println!("proton:       {}", s.proton_path());
    println!("launch:       {:?} + {:?}", s.default_env(), s.launch_settings());
    println!("fika:         {}", install.fika_installed());
    println!("patches:      {}", install.patches_dir().display());
    for p in proton::list(&s.extra_proton_paths()) {
        println!("proton build: [{}] {}", p.kind, p.name);
    }
    println!("steam eft:    {:?}", steam::find_eft().map(|g| g.game_dir));
    if let Some(host) = std::env::args().nth(1) {
        let c = Client::new(&host);
        println!("ping {host}: {:?}", c.ping());
    }
}
