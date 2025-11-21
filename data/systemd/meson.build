systemd_user_unit_dir = get_option('systemd_user_unit_dir')
systemd_dep = dependency('systemd', required: false)
if systemd_user_unit_dir == ''
  if systemd_dep.found()
    systemd_user_unit_dir = systemd_dep.get_variable('systemd_user_unit_dir')
  else
    systemd_user_unit_dir = prefix / libdir / 'systemd' / 'user'
  endif
endif

configure_file(
  input: f'@app_id@.service.in',
  output: f'@app_id@.service',
  configuration: serviceconf,
  install_dir: systemd_user_unit_dir,
)
