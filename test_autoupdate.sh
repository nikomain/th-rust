#!/bin/bash

echo "🧪 Testing Autoupdate Functionality"
echo "===================================="
echo

# Source the wrapper function
source th.sh

echo "📋 Test Steps:"
echo "1. Clear any existing update cache"
echo "2. Run a command to trigger background update check"
echo "3. Wait for cache to populate"
echo "4. Run another command to see update notification"
echo "5. Test the 'th update' command"
echo

echo "🧹 Step 1: Clearing update cache..."
th clear-update-cache
echo

echo "⏳ Step 2: Running 'th version' to trigger update check..."
echo "(This will check for updates in background - 5 second cache in test mode)"
th version
echo

echo "⏱️  Step 3: Waiting 6 seconds for cache to populate..."
sleep 6
echo

echo "🔔 Step 4: Running 'th version' again to see update notification..."
th version
echo

echo "📦 Step 5: Testing 'th update' command..."
th update
echo

echo "🔄 Step 6: Running command again to show no notification after update..."
sleep 1
th version
echo

echo "✅ Test completed! You should have seen:"
echo "  - First run: No update notification"
echo "  - Second run: Update notification (1.5.0 → 1.6.0)" 
echo "  - Update command: Simulated download and install"
echo "  - Final run: No notification (already up to date)"