import json
import struct

def align_to_4bytes(offset):
    return (offset + 3) & ~3

# Read the GLTF JSON
with open('cube.gltf', 'r') as f:
    gltf = json.load(f)

# Read the binary data
with open('cube.bin', 'rb') as f:
    bin_data = f.read()

# Update the buffer to be internal
gltf['buffers'][0] = {
    'byteLength': len(bin_data)
}

# Convert GLTF to JSON bytes with UTF-8 encoding
json_bytes = json.dumps(gltf).encode('utf-8')
json_length = len(json_bytes)
# Pad to 4-byte boundary
json_pad = (4 - (json_length % 4)) % 4
json_bytes += b' ' * json_pad

# Calculate total file size
bin_length = len(bin_data)
bin_pad = (4 - (bin_length % 4)) % 4
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
    f.write(bin_data)
    f.write(b'\0' * bin_pad) 