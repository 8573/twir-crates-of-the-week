with import <nixpkgs> {};

stdenv.mkDerivation rec {
  name = "twir-cotw-list.rs";

  nativeBuildInputs = [
    asciidoctor
    cargo
    clang
    rustc
    rustfmt
  ];

  buildInputs = [
  ];

  lib_path = lib.makeLibraryPath buildInputs;

  postFixup = ''
    for f in target/*/"$name"; do
      patchelf --set-rpath "$lib_path" "$f"
    done
  '';
}
