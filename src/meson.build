pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()
gnome = import('gnome')

compiled_resources = gnome.compile_resources(
  'pfs',
  'pfs.gresource.xml',
  gresource_bundle: true,
  install_dir: pkgdatadir,
)

conf = configuration_data()
conf.set_quoted('VERSION', meson.project_version())
conf.set_quoted('GETTEXT_PACKAGE', 'pfs')
conf.set_quoted('LOG_DOMAIN', 'pfs')
conf.set_quoted('LOCALEDIR', get_option('prefix') / get_option('localedir'))
conf.set_quoted('PKGDATADIR', pkgdatadir)

configure_file(input: 'config.rs.in', output: 'config.rs', configuration: conf)

# Copy the config.rs output to the source directory.
run_command(
  'cp',
  meson.project_build_root() / 'src' / 'config.rs',
  meson.project_source_root() / 'src' / 'config.rs',
  check: true,
)

cargo_bin = find_program('cargo')
cargo_opt = ['--manifest-path', meson.project_source_root() / 'Cargo.toml']
cargo_opt += ['--target-dir', meson.project_build_root() / 'src']
cargo_env = ['CARGO_HOME=' + meson.project_build_root() / 'cargo-home']
cargo_env += ['PFS_RESOURCE_DIR=' + meson.current_build_dir()]

if get_option('debug')
  rust_target = 'debug'
else
  cargo_opt += ['--release']
  rust_target = 'release'
endif

pfs_inc = include_directories('include/')
libpfs_deps = [libadwaita_dep, gtk_dep, gobject_dep]

cargo_static_lib_build = custom_target(
  'cargo-build-static-lib',
  build_by_default: true,
  build_always_stale: true,
  output: 'libpfs.a',
  console: true,
  command: [
    'env',
    cargo_env,
    cargo_bin,
    'build',
    '--lib',
    cargo_opt,
    '&&',
    'cp',
    meson.project_build_root() / 'src' / rust_target / 'libpfs.a',
    '@OUTPUT@',
  ],
  depends: compiled_resources,
)

if get_option('shared-lib')

  install_headers('include' / 'pfs.h', subdir: 'pfs-0')

  # This is the same build as above but we install the lib this time and
  # patch the soname: https://github.com/rust-lang/cargo/issues/5045
  patchelf_bin = find_program('patchelf')
  cargo_lib_build = custom_target(
    'cargo-build-lib',
    build_by_default: true,
    build_always_stale: true,
    output: 'libpfs-0.so',
    console: true,
    install: true,
    install_dir: get_option('libdir'),
    command: [
      'env',
      cargo_env,
      cargo_bin,
      'build',
      '--lib',
      cargo_opt,
      '&&',
      'cp',
      meson.project_build_root() / 'src' / rust_target / 'libpfs.so',
      meson.current_build_dir(),
      '&&',
      'cp',
      meson.project_build_root() / 'src' / 'libpfs.so',
      '@OUTPUT@',
      '&&',
      patchelf_bin,
      '--set-soname',
      'libpfs-0.so',
      '@OUTPUT@',
    ],
    depends: compiled_resources,
  )

  pkg = pkgconfig.generate(
    name: 'pfs',
    filebase: 'libpfs-0',
    version: meson.project_version(),
    subdirs: 'pfs-0',
    description: 'Phosh File Selector',
    libraries: '-lpfs-0',
    requires_private: libpfs_deps,
  )

endif

# Dependency for superprojects
pfs_dep = declare_dependency(
  include_directories: pfs_inc,
  dependencies: libpfs_deps,
  link_with: cargo_static_lib_build,
)

subdir('examples')
