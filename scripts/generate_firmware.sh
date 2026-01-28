#!/bin/bash
# Firmware Generation Script
#
# This script builds an application and generates a firmware.bin file
# suitable for bootloader upgrade.
#
# Usage:
#   ./scripts/generate_firmware.sh [example_name]
#
# Example:
#   ./scripts/generate_firmware.sh led_blink_app

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
EXAMPLE_NAME="${1:-led_blink_app}"
OUTPUT_DIR="target/firmware"
FIRMWARE_FILE="firmware.bin"
MAX_SIZE=$((448 * 1024))  # 448KB max

echo -e "${GREEN}=== Firmware Generation Script ===${NC}"
echo "Example: $EXAMPLE_NAME"
echo "Output: $OUTPUT_DIR/$FIRMWARE_FILE"
echo ""

# Check if example exists
if [ ! -f "examples/${EXAMPLE_NAME}.rs" ]; then
    echo -e "${RED}Error: Example 'examples/${EXAMPLE_NAME}.rs' not found${NC}"
    exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Step 1: Build the example
echo -e "${YELLOW}Step 1: Building example...${NC}"
cargo build --example "$EXAMPLE_NAME" --release

if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi

echo -e "${GREEN}Build successful${NC}"
echo ""

# Step 2: Generate binary file
echo -e "${YELLOW}Step 2: Generating binary file...${NC}"
cargo objcopy --example "$EXAMPLE_NAME" --release -- -O binary "$OUTPUT_DIR/$FIRMWARE_FILE"

if [ $? -ne 0 ]; then
    echo -e "${RED}Binary generation failed!${NC}"
    exit 1
fi

echo -e "${GREEN}Binary generated${NC}"
echo ""

# Step 3: Check file size
echo -e "${YELLOW}Step 3: Checking firmware size...${NC}"
FIRMWARE_SIZE=$(stat -f%z "$OUTPUT_DIR/$FIRMWARE_FILE" 2>/dev/null || stat -c%s "$OUTPUT_DIR/$FIRMWARE_FILE" 2>/dev/null)

echo "Firmware size: $FIRMWARE_SIZE bytes ($((FIRMWARE_SIZE / 1024)) KB)"
echo "Maximum size:  $MAX_SIZE bytes ($((MAX_SIZE / 1024)) KB)"

if [ "$FIRMWARE_SIZE" -gt "$MAX_SIZE" ]; then
    echo -e "${RED}Error: Firmware size exceeds maximum!${NC}"
    exit 1
fi

echo -e "${GREEN}Size check passed${NC}"
echo ""

# Step 4: Calculate CRC32
echo -e "${YELLOW}Step 4: Calculating CRC32...${NC}"

# Try different CRC32 tools
if command -v crc32 &> /dev/null; then
    CRC32=$(crc32 "$OUTPUT_DIR/$FIRMWARE_FILE")
elif command -v cksum &> /dev/null; then
    CRC32=$(cksum "$OUTPUT_DIR/$FIRMWARE_FILE" | awk '{print $1}')
elif command -v python3 &> /dev/null; then
    CRC32=$(python3 -c "import zlib; print(hex(zlib.crc32(open('$OUTPUT_DIR/$FIRMWARE_FILE', 'rb').read())))")
else
    echo -e "${YELLOW}Warning: No CRC32 tool found, skipping checksum${NC}"
    CRC32="N/A"
fi

echo "CRC32: $CRC32"
echo "$CRC32" > "$OUTPUT_DIR/firmware.crc"
echo ""

# Step 5: Generate info file
echo -e "${YELLOW}Step 5: Generating firmware info...${NC}"
cat > "$OUTPUT_DIR/firmware.info" <<EOF
Firmware Information
====================
Example:     $EXAMPLE_NAME
Build Date:  $(date)
Size:        $FIRMWARE_SIZE bytes ($((FIRMWARE_SIZE / 1024)) KB)
CRC32:       $CRC32
Max Size:    $MAX_SIZE bytes ($((MAX_SIZE / 1024)) KB)
Target:      STM32F407VG
Slot:        A or B (448KB each)
EOF

cat "$OUTPUT_DIR/firmware.info"
echo ""

# Step 6: Summary
echo -e "${GREEN}=== Firmware Generation Complete ===${NC}"
echo ""
echo "Files generated:"
echo "  - $OUTPUT_DIR/$FIRMWARE_FILE (binary)"
echo "  - $OUTPUT_DIR/firmware.crc (checksum)"
echo "  - $OUTPUT_DIR/firmware.info (metadata)"
echo ""
echo "Next steps:"
echo "  1. Copy $OUTPUT_DIR/$FIRMWARE_FILE to SD card root directory"
echo "  2. Insert SD card into device"
echo "  3. Press and hold upgrade button (PE4)"
echo "  4. Reset the device"
echo "  5. Observe LED status during upgrade"
echo ""
echo -e "${GREEN}Done!${NC}"
