project(
  'PhoshFileSelector',
  ['rust', 'c'],
  version: '0.0.5',
  meson_version: '>= 1.0.0',
  default_options: ['warning_level=2', 'werror=false'],
)

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

gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: get_option('examples'),
  update_desktop_database: get_option('examples'),
)
