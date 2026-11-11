// Génère l'icône .ico (exe + installeur) à partir du même dessin que l'appli
// et l'intègre à l'exe via windres.
#[allow(dead_code)]
#[path = "src/icon.rs"]
mod icon;

use std::{env, fs, path::PathBuf, process::Command};

fn ico(sizes: &[u32]) -> Vec<u8> {
    let images: Vec<(u32, Vec<u8>)> = sizes
        .iter()
        .map(|&s| {
            let px = icon::render(s, icon::ON);
            let mut d = Vec::new();
            d.extend(40u32.to_le_bytes());
            d.extend((s as i32).to_le_bytes());
            d.extend((s as i32 * 2).to_le_bytes());
            d.extend(1u16.to_le_bytes());
            d.extend(32u16.to_le_bytes());
            d.extend([0u8; 24]);
            for y in (0..s).rev() {
                for x in 0..s {
                    let [r, g, b, a] = px[(y * s + x) as usize];
                    d.extend([b, g, r, a]);
                }
            }
            d.extend(vec![0u8; ((s as usize + 31) / 32 * 4) * s as usize]);
            (s, d)
        })
        .collect();

    let mut out = Vec::new();
    out.extend(0u16.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend((images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (s, d) in &images {
        let dim = if *s >= 256 { 0 } else { *s as u8 };
        out.extend([dim, dim, 0, 0]);
        out.extend(1u16.to_le_bytes());
        out.extend(32u16.to_le_bytes());
        out.extend((d.len() as u32).to_le_bytes());
        out.extend((offset as u32).to_le_bytes());
        offset += d.len();
    }
    for (_, d) in images {
        out.extend(d);
    }
    out
}

fn main() {
    println!("cargo:rerun-if-changed=src/icon.rs");
    println!("cargo:rerun-if-changed=build.rs");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let data = ico(&[16, 20, 24, 32, 40, 48, 64, 128]);
    fs::write(out.join("hdr.ico"), &data).unwrap();
    let dist = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("dist");
    fs::create_dir_all(&dist).unwrap();
    fs::write(dist.join("hdr.ico"), &data).unwrap();

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        fs::write(out.join("app.rc"), "1 ICON \"hdr.ico\"\n").unwrap();
        let ok = Command::new("x86_64-w64-mingw32-windres")
            .current_dir(&out)
            .args(["-O", "coff", "-i", "app.rc", "-o", "res.o"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "windres a échoué");
        println!("cargo:rustc-link-arg-bins={}", out.join("res.o").display());
    }
}
