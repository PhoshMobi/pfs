project(
  'PhoshFileSelector',
  ['rust', 'c'],
  version: '0.0.6',
  meson_version: '>= 1.1.0',
  default_options: ['warning_level=2', 'werror=false'],
)

prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
datadir = prefix / get_option('datadir')
libdir = prefix / get_option('libdir')
servicedir = datadir / 'dbus-1' / 'services'

i18n = import('i18n')
gnome = import('gnome')
pkgconfig = import('pkgconfig')

gtk_dep = dependency('gtk4', version: '>=4.14')
libadwaita_dep = dependency('libadwaita-1', version: '>=1.4')
gobject_dep = dependency('gobject-2.0', version: '>=2.70')

subdir('data')
subdir('src')
subdir('po')

run_data = configuration_data()
run_data.set('ABS_BUILDDIR', meson.current_build_dir())
configure_file(input: 'run.in', output: 'run', configuration: run_data)

summary({'Examples': get_option('examples')}, bool_yn: true)

gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: get_option('examples'),
  update_desktop_database: get_option('examples'),
)
