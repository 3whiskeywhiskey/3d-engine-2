# shell.nix
{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Basic build tools
    pkg-config
    cmake
    
    # X11 dependencies
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    xorg.libXinerama
    xorg.libxcb
    xorg.libXrender
    xorg.libXfixes
    xorg.libXext
    xorg.libXtst
    
    # Graphics dependencies
    vulkan-loader
    vulkan-tools
    vulkan-headers
    vulkan-validation-layers
    mesa.drivers
    nvidia-vaapi-driver
    
    # Other dependencies that might be needed
    libxkbcommon.dev
    libxkbcommon.out
    wayland
  ];

  shellHook = ''
    # Library paths
    export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath [
      pkgs.vulkan-loader
      pkgs.xorg.libX11
      pkgs.xorg.libXcursor
      pkgs.xorg.libXrandr
      pkgs.xorg.libXi
      pkgs.xorg.libXinerama
      pkgs.xorg.libxcb
      pkgs.xorg.libXrender
      pkgs.xorg.libXfixes
      pkgs.xorg.libXext
      pkgs.xorg.libXtst
      pkgs.libxkbcommon
      pkgs.mesa.drivers
    ]}"
    
    # Vulkan configuration
    export VK_LAYER_PATH="${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d"
    export VK_ICD_FILENAMES="/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.x86_64.json"
    
    # Debug settings
    export RUST_LOG="wgpu=trace,vulkan=trace,winit=trace"
    export RUST_BACKTRACE="full"
    export VK_LOADER_DEBUG=all
    export LIBGL_DEBUG=verbose
    export WINIT_UNIX_BACKEND=x11
    
    # NVIDIA specific
    export __GL_SHADER_DISK_CACHE=1
    export __GL_THREADED_OPTIMIZATIONS=1
    export __GLX_VENDOR_LIBRARY_NAME=nvidia
  '';
} 