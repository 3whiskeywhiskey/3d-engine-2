import json
import struct
import base64
from pathlib import Path

def align_to_4bytes(offset):
    return (offset + 3) & ~3

# Read the GLTF JSON
with open('cube.gltf', 'r') as f:
    gltf = json.load(f)

# Read the binary data
with open('cube.bin', 'rb') as f:
    bin_data = f.read()

# Read the texture data
with open('cube_texture.png', 'rb') as f:
    texture_data = f.read()

# Update the buffer to be internal
gltf['buffers'][0] = {
    'byteLength': len(bin_data)
}

# Update the image to be a buffer view
texture_buffer_view = {
    'buffer': 0,
    'byteOffset': len(bin_data),
    'byteLength': len(texture_data)
}

gltf['bufferViews'].append(texture_buffer_view)
gltf['images'][0] = {
    'bufferView': len(gltf['bufferViews']) - 1,
    'mimeType': 'image/png'
}

# Convert GLTF to JSON bytes with UTF-8 encoding
json_bytes = json.dumps(gltf).encode('utf-8')
json_length = len(json_bytes)
# Pad to 4-byte boundary
json_pad = (4 - (json_length % 4)) % 4
json_bytes += b' ' * json_pad

# Combine and pad binary data
combined_bin = bin_data + texture_data
bin_length = len(combined_bin)
bin_pad = (4 - (bin_length % 4)) % 4
combined_bin += b'\0' * bin_pad

# Calculate total file size
total_size = 12 + 8 + json_length + json_pad + 8 + bin_length + bin_pad

# Write the GLB file
with open('cube.glb', 'wb') as f:
    # Write GLB header
    f.write(struct.pack('<4sII', b'glTF', 2, total_size))
    
    # Write JSON chunk
    f.write(struct.pack('<II', json_length + json_pad, 0x4E4F534A))  # 'JSON'
    f.write(json_bytes)
    
    # Write BIN chunk
    f.write(struct.pack('<II', bin_length + bin_pad, 0x004E4942))  # 'BIN\0'
    f.write(combined_bin)
    f.write(b'\0' * bin_pad) 