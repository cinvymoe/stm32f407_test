#!/bin/bash
# Bootloader Upgrade Test Script
#
# This script automates the testing of the bootloader upgrade process.
# It builds firmware, flashes the bootloader, and provides instructions
# for manual testing.
#
# Usage:
#   ./scripts/test_upgrade.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Bootloader Upgrade Test Script ===${NC}"
echo ""

# Check if probe-rs is installed
if ! command -v probe-rs &> /dev/null; then
    echo -e "${RED}Error: probe-rs not found${NC}"
    echo "Install with: cargo install probe-rs --features cli"
    exit 1
fi

# Step 1: Build bootloader
echo -e "${YELLOW}Step 1: Building bootloader...${NC}"
cargo build --release

if [ $? -ne 0 ]; then
    echo -e "${RED}Bootloader build failed!${NC}"
    exit 1
fi

echo -e "${GREEN}Bootloader built successfully${NC}"
echo ""

# Step 2: Generate test firmware
echo -e "${YELLOW}Step 2: Generating test firmware...${NC}"
./scripts/generate_firmware.sh led_blink_app

if [ $? -ne 0 ]; then
    echo -e "${RED}Firmware generation failed!${NC}"
    exit 1
fi

echo -e "${GREEN}Test firmware generated${NC}"
echo ""

# Step 3: Flash bootloader
echo -e "${YELLOW}Step 3: Flashing bootloader to device...${NC}"
echo "Target: STM32F407VGTx"
echo "Protocol: SWD"
echo ""

read -p "Connect STM32F407 and press Enter to continue (or Ctrl+C to cancel)..."

probe-rs run --chip STM32F407VGTx --protocol swd target/thumbv7em-none-eabihf/release/stm32f407-sd-bootloader

if [ $? -ne 0 ]; then
    echo -e "${RED}Flashing failed!${NC}"
    exit 1
fi

echo -e "${GREEN}Bootloader flashed successfully${NC}"
echo ""

# Step 4: Manual test instructions
echo -e "${BLUE}=== Manual Testing Instructions ===${NC}"
echo ""
echo "1. Prepare SD Card:"
echo "   - Format SD card as FAT32"
echo "   - Copy target/firmware/firmware.bin to SD card root"
echo "   - Safely eject SD card"
echo ""
echo "2. Insert SD Card:"
echo "   - Insert SD card into the device"
echo ""
echo "3. Trigger Upgrade:"
echo "   - Press and hold the upgrade button (PE4)"
echo "   - Press the reset button (or power cycle)"
echo "   - Keep holding PE4 for 2 seconds"
echo ""
echo "4. Observe LED Status:"
echo "   - LED2 (PA6): Slow blink = Bootloader running"
echo "   - LED1 (PA7): Medium blink (200ms) = Reading firmware"
echo "   - LED1 (PA7): Slow blink (500ms) = Writing firmware"
echo "   - LED1 (PA7): Solid on = Upgrade successful"
echo "   - LED1 (PA7): Fast blink (100ms) = Upgrade failed"
echo ""
echo "5. Verify Application:"
echo "   - After successful upgrade, device should reboot"
echo "   - LED blink application should start running"
echo "   - PA5 LED should blink at 1Hz"
echo ""
echo "6. Test Scenarios:"
echo ""
echo "   a) Normal Upgrade:"
echo "      - Follow steps 1-5 above"
echo "      - Verify successful upgrade"
echo ""
echo "   b) No SD Card:"
echo "      - Remove SD card"
echo "      - Reset device without pressing PE4"
echo "      - Should boot directly to application"
echo ""
echo "   c) Invalid Firmware:"
echo "      - Create a corrupted firmware.bin (truncate or modify)"
echo "      - Attempt upgrade"
echo "      - Should fail with fast LED blink"
echo "      - Should boot to existing application"
echo ""
echo "   d) Firmware Too Large:"
echo "      - Create a firmware.bin > 448KB"
echo "      - Attempt upgrade"
echo "      - Should reject with fast LED blink"
echo ""
echo "7. Monitor Logs (Optional):"
echo "   - Connect probe-rs for RTT logging:"
echo "   - probe-rs run --chip STM32F407VGTx target/thumbv7em-none-eabihf/release/stm32f407-sd-bootloader"
echo ""
echo -e "${GREEN}=== Test Setup Complete ===${NC}"
echo ""
echo "Bootloader is flashed and ready for testing."
echo "Follow the manual testing instructions above."
echo ""
