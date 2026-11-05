//! Branding regression: graphical resources and compatibility identities stay in sync.
#[test]
fn fox_logo_has_transparent_padding_and_real_content() {
    let png = image::load_from_memory(include_bytes!("../assets/xiaodaji.png")).unwrap().to_rgba8();
    assert_eq!(png.dimensions(), (1024, 1024));
    for (x, y) in [(0, 0), (1023, 0), (0, 1023), (1023, 1023)] {
        assert_eq!(png.get_pixel(x, y)[3], 0);
    }
    let visible = png.pixels().filter(|pixel| pixel[3] > 200).count();
    assert!(visible > 300_000 && visible < 850_000);
    let icon = daozai::theme::app_icon();
    assert_eq!((icon.width, icon.height), (128, 128));
    assert_eq!(icon.rgba.len(), 128 * 128 * 4);
    assert!(icon.rgba.chunks_exact(4).any(|p| p[0] > 200 && p[1] < 100 && p[3] > 200));
}

#[test]
fn ico_includes_small_and_high_dpi_sizes_with_valid_offsets() {
    let ico = include_bytes!("../assets/xiaodaji.ico");
    assert_eq!(&ico[..4], &[0, 0, 1, 0]);
    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    let mut sizes = Vec::new();
    for i in 0..count {
        let at = 6 + i * 16;
        let width = if ico[at] == 0 { 256 } else { ico[at] as usize };
        let height = if ico[at + 1] == 0 { 256 } else { ico[at + 1] as usize };
        assert_eq!(width, height);
        let length = u32::from_le_bytes(ico[at+8..at+12].try_into().unwrap()) as usize;
        let offset = u32::from_le_bytes(ico[at+12..at+16].try_into().unwrap()) as usize;
        assert!(length > 0 && offset >= 6 + count * 16 && offset + length <= ico.len());
        sizes.push(width);
    }
    assert_eq!(sizes, [16, 20, 24, 32, 40, 48, 64, 128, 256]);
}

#[test]
fn chinese_branding_does_not_change_legacy_install_or_data_identity() {
    let installer = include_str!("../installer/setup.nsi");
    assert!(installer.contains("!define APP_NAME \"小妲己\""));
    assert!(installer.contains("!define APP_ID \"Daozai\""));
    assert!(installer.contains("assets\\xiaodaji.ico"));
    assert!(installer.contains("cleanup-legacy-links.ps1"));
    assert!(include_str!("../installer/cleanup-legacy-links.ps1").contains("$shell.CreateShortcut($path).TargetPath"));
    let package = include_str!("../scripts/package.ps1");
    assert!(package.contains("小妲己-v$version-windows-x64"));
    assert!(package.contains("小妲己-安装包-v$version-x64.exe"));
    assert!(package.contains("[Text.UTF8Encoding]::new($false)"));
    assert!(!package.contains("-Encoding ascii"));
    assert!(include_str!("../src/model.rs").contains("join(\"Daozai\")"));
    assert!(include_str!("../src/mail.rs").contains("\"Daozai.Mail\""));
}

#[test]
fn package_lock_and_windows_resources_match_the_compiled_version() {
    let version = env!("CARGO_PKG_VERSION");
    let version_line = format!("version = \"{version}\"");
    assert!(include_str!("../Cargo.toml").lines().any(|line| line == version_line));
    let lock = include_str!("../Cargo.lock");
    let package = lock.split("[[package]]").find(|block| block.lines().any(|line| line == "name = \"daozai\"" )).unwrap();
    assert!(package.lines().any(|line| line == version_line));
    let numeric = format!("{},0", version.replace('.', ","));
    let resources = include_str!("../app.rc");
    assert!(resources.contains(&format!("FILEVERSION {numeric}")));
    assert!(resources.contains(&format!("PRODUCTVERSION {numeric}")));
    assert!(resources.contains(&format!("VALUE \"ProductVersion\", \"{version}\"")));
    assert!(resources.contains(&format!("VALUE \"FileVersion\", \"{version}\"")));
    let readme = include_str!("../README.md");
    assert!(readme.contains(&format!("小妲己-安装包-v{version}-x64.exe")));
    assert!(readme.contains(&format!("小妲己-v{version}-windows-x64.zip")));
}

#[test]
fn distributed_executables_and_all_installer_paths_use_chinese_names() {
    let resources = include_str!("../app.rc");
    assert!(resources.contains("VALUE \"OriginalFilename\", \"小妲己.exe\""));
    let installer = include_str!("../installer/setup.nsi");
    assert!(installer.contains("!define APP_EXE \"小妲己.exe\""));
    assert!(installer.contains("!define UNINSTALL_EXE \"卸载小妲己.exe\""));
    assert!(installer.contains("WriteUninstaller \"$INSTDIR\\${UNINSTALL_EXE}\""));
    assert!(installer.contains("Delete \"$INSTDIR\\daozai.exe\""));
    assert!(installer.contains("Delete \"$INSTDIR\\Uninstall.exe\""));
    let package = include_str!("../scripts/package.ps1");
    assert!(package.contains("Join-Path $stagingDirectory '小妲己.exe'"));
    assert!(package.contains("check-icons.ps1"));
    assert!(!package.contains("Join-Path $stagingDirectory 'daozai.exe'"));
    let updater = include_str!("../installer/update.ps1");
    assert!(updater.contains("windows-x64/小妲己.exe"));
    assert!(updater.contains("windows-x64/daozai.exe"));
    assert!(updater.contains("$entries.Count -ne 1"));
}
