if not get_option('examples')
  subdir_done()
endif

foreach example : examples

  scalable_dir = 'hicolor' / 'scalable' / 'apps'
  install_data(
    scalable_dir / ('@0@.svg').format(example[0]),
    install_dir: get_option('datadir') / 'icons' / scalable_dir,
  )

endforeach
