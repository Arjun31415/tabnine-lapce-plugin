xdg_data_dir :=  "$HOME/.local/share"
plugin_dir := "plugins/Arjun31415.lapce-tabine"

build:
    cargo make

install-stable: build
    mkdir -p {{xdg_data_dir}}/lapce-stable/{{plugin_dir}}/bin
    yes | cp  bin/lapce-tabnine.wasm {{xdg_data_dir}}/lapce-stable/{{plugin_dir}}/bin
    yes | cp  volt.toml {{xdg_data_dir}}/lapce-stable/{{plugin_dir}}/

