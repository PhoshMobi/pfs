examples = [['mobi.phosh.FileSelectorDemo', 'pfs-demo'], ['mobi.phosh.FileOpen', 'pfs-open']]
merged = []

install_data('mobi.phosh.FileSelector.gschema.xml', install_dir: datadir / 'glib-2.0' / 'schemas')

compile_schemas = find_program('glib-compile-schemas', required: false)
if compile_schemas.found()
  test(
    'Validate schema file',
    compile_schemas,
    args: ['--strict', '--dry-run', meson.current_source_dir()],
  )
endif
compiled = gnome.compile_schemas(build_by_default: true)

#
# Examples
#

if not get_option('examples')
  subdir_done()
endif

foreach example : examples

  desktopconf = configuration_data()
  desktopconf.set('DEMO', example[1])

  merged += i18n.merge_file(
    input: configure_file(
      input: example[0] + '.desktop.in.in',
      output: example[0] + '.desktop.in',
      configuration: desktopconf,
    ),
    output: example[0] + '.desktop',
    po_dir: '../po',
    install: get_option('examples'),
    install_dir: datadir / 'applications',
    type: 'desktop',
  )
endforeach

desktop_utils = find_program('desktop-file-validate', required: false)
if desktop_utils.found()
  test('Validate desktop file', desktop_utils, args: merged)
endif

filemanager1 = 'org.freedesktop.FileManager1'
app_id = examples[1][0]
serviceconf = configuration_data()
serviceconf.set('app_id', app_id)
serviceconf.set('bindir', bindir)
serviceconf.set('busname', filemanager1)
serviceconf.set('bin', examples[1][1])
configure_file(
  input: f'@app_id@.service.in',
  output: f'@app_id@.service',
  configuration: serviceconf,
  install_dir: servicedir,
)

subdir('systemd')
subdir('icons')
