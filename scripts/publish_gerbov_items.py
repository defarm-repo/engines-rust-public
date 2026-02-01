#!/usr/bin/env python3
"""
Publish Gerbov items to make them visible on public circuit page
"""

import requests
import json

# API Configuration
BASE_URL = "https://defarm-engines-api-production.up.railway.app"
USERNAME = "gerbov"
PASSWORD = "Gerbov2024!Test"
CIRCUIT_ID = "4eb4e8da-12f7-4bfb-9610-686e9c21c1a2"

def main():
    print("="*80)
    print("📢 PUBLISHING GERBOV ITEMS")
    print("="*80)

    # Login
    print(f"\n🔐 Authenticating as {USERNAME}...")
    resp = requests.post(f"{BASE_URL}/api/auth/login", json={"username": USERNAME, "password": PASSWORD})
    if resp.status_code != 200:
        print(f"❌ Authentication failed: {resp.text}")
        return

    token = resp.json()["token"]
    headers = {"Authorization": f"Bearer {token}"}
    print("✅ Authenticated!")

    # Get current circuit public settings
    print(f"\n📋 Getting current circuit settings...")
    resp = requests.get(f"{BASE_URL}/api/circuits/{CIRCUIT_ID}/public-settings", headers=headers)

    if resp.status_code == 200:
        current_settings = resp.json()
        print(f"✅ Current settings retrieved")
        print(f"   Auto-publish: {current_settings.get('auto_publish_pushed_items', False)}")
        print(f"   Published items: {len(current_settings.get('published_items', []))}")
    else:
        print(f"⚠️  No public settings found, will create new ones")
        current_settings = {
            "access_mode": "Public",
            "auto_publish_pushed_items": True,
            "published_items": []
        }

    # Get all items in circuit
    print(f"\n📦 Getting all items in circuit...")
    resp = requests.get(f"{BASE_URL}/api/circuits/{CIRCUIT_ID}/items", headers=headers)

    if resp.status_code != 200:
        print(f"❌ Failed to get items: {resp.text}")
        return

    items = resp.json()
    print(f"✅ Found {len(items)} items in circuit")

    # Extract DFIDs
    dfids = [item["dfid"] for item in items if "dfid" in item]
    print(f"   DFIDs to publish: {len(dfids)}")

    # Update public settings with all items published
    print(f"\n🚀 Publishing all items...")

    # Merge with existing published_items to avoid duplicates
    existing_published = set(current_settings.get("published_items", []))
    all_published = list(existing_published.union(set(dfids)))

    updated_settings = {
        "access_mode": "Public",
        "auto_publish_pushed_items": True,  # Enable for future items
        "published_items": all_published,
        "show_encrypted_events": False,
        "auto_approve_members": False
    }

    # Add optional fields if they exist
    for field in ["public_name", "public_description", "tagline", "primary_color", "secondary_color", "logo_url", "footer_text"]:
        if field in current_settings and current_settings[field]:
            updated_settings[field] = current_settings[field]

    # If no custom name/description, add defaults
    if "public_name" not in updated_settings:
        updated_settings["public_name"] = "Gerbov - Fazenda Santa Fé"
    if "public_description" not in updated_settings:
        updated_settings["public_description"] = "Rastreabilidade de bovinos da Fazenda Santa Fé com SISBOV e RFID"
    if "tagline" not in updated_settings:
        updated_settings["tagline"] = "Gestão transparente de rebanho bovino"

    resp = requests.put(
        f"{BASE_URL}/api/circuits/{CIRCUIT_ID}/public-settings",
        headers=headers,
        json=updated_settings
    )

    if resp.status_code in [200, 201]:
        print("✅ Items published successfully!")
        print(f"\n📊 Summary:")
        print(f"   Total items in circuit: {len(items)}")
        print(f"   Items published: {len(all_published)}")
        print(f"   Auto-publish enabled: True")
        print(f"\n🌐 View public page at:")
        print(f"   https://circuits.defarm.net/public/{CIRCUIT_ID}")
        print(f"\n🔗 Or access circuit normally at:")
        print(f"   https://circuits.defarm.net/circuits/{CIRCUIT_ID}")
    else:
        print(f"❌ Failed to publish items: {resp.status_code}")
        print(resp.text)

if __name__ == "__main__":
    main()
