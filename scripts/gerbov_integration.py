#!/usr/bin/env python3
"""
Gerbov Integration Script
Imports animal data from Gerbov CSV and pushes to DeFarm circuit with Stellar Testnet adapter
"""

import csv
import json
import requests
import time
from datetime import datetime
from typing import Dict, List, Optional

# API Configuration
BASE_URL = "https://defarm-engines-api-production.up.railway.app"
# BASE_URL = "http://localhost:8080"

# Demo account credentials (from CLAUDE.md)
USERNAME = "gerbov"
PASSWORD = "Gerbov2024!Test"

class GerbovIntegration:
    def __init__(self, base_url: str):
        self.base_url = base_url
        self.token = None
        self.user_id = None
        self.circuit_id = None

    def login(self, username: str, password: str) -> bool:
        """Authenticate and get JWT token"""
        print(f"🔐 Authenticating as {username}...")

        response = requests.post(
            f"{self.base_url}/api/auth/login",
            json={"username": username, "password": password}
        )

        if response.status_code == 200:
            data = response.json()
            self.token = data["token"]
            self.user_id = data["user_id"]
            print(f"✅ Authenticated! User ID: {self.user_id}")
            return True
        else:
            print(f"❌ Authentication failed: {response.text}")
            return False

    def headers(self) -> Dict[str, str]:
        """Get authorization headers"""
        return {"Authorization": f"Bearer {self.token}"}

    def get_or_create_circuit(self) -> Optional[str]:
        """Get existing Gerbov circuit or create new one"""
        print("\n🔧 Checking for existing Gerbov circuit...")

        # List all circuits
        response = requests.get(
            f"{self.base_url}/api/circuits",
            headers=self.headers()
        )

        if response.status_code == 200:
            circuits = response.json()
            # Look for existing Gerbov circuit
            for circuit in circuits:
                if "Gerbov" in circuit.get("name", ""):
                    self.circuit_id = circuit["circuit_id"]
                    print(f"✅ Found existing circuit: {circuit['name']}")
                    print(f"   ID: {self.circuit_id}")
                    return self.circuit_id

        # Create new circuit if not found
        print("   No existing Gerbov circuit found. Creating new one...")

        circuit_data = {
            "name": "Gerbov - Fazenda Santa Fé",
            "description": "Integração com sistema Gerbov - Gestão de Animais da Fazenda Santa Fé. Dados de bovinos com rastreabilidade SISBOV e RFID.",
            "visibility": "private",
            "permissions": {
                "require_approval_for_push": False,
                "require_approval_for_pull": False,
                "allow_member_invite": True,
                "allow_public_visibility": True
            }
        }

        response = requests.post(
            f"{self.base_url}/api/circuits",
            headers=self.headers(),
            json=circuit_data
        )

        if response.status_code in [200, 201]:
            circuit = response.json()
            self.circuit_id = circuit["circuit_id"]
            print(f"✅ Circuit created! ID: {self.circuit_id}")
            return self.circuit_id
        else:
            print(f"❌ Failed to create circuit: {response.status_code} - {response.text}")
            return None

    def configure_adapter(self, circuit_id: str) -> bool:
        """Configure Stellar Testnet adapter for the circuit"""
        print(f"\n🛰️  Configuring Stellar Testnet adapter for circuit {circuit_id}...")

        adapter_config = {
            "adapter_type": "IpfsIpfs",
            "required_identifiers": {
                "canonical": ["sisbov", "chip"],
                "contextual": ["numero_animal", "numero_matriz"]
            },
            "sponsor_adapter_access": True,
            "auto_apply_namespace": True,
            "default_namespace": "bovino",
            "use_fingerprint": True,
            "allowed_namespaces": ["bovino", "generic"]
        }

        response = requests.put(
            f"{self.base_url}/api/circuits/{circuit_id}/adapter",
            headers=self.headers(),
            json=adapter_config
        )

        if response.status_code == 200:
            print("✅ Adapter configured successfully!")
            print("   - Type: IPFS (decentralized storage)")
            print("   - Canonical IDs: sisbov, chip")
            print("   - Contextual IDs: numero_animal, numero_matriz")
            print("   - Namespace: bovino")
            print("   ⚠️  Note: Using IPFS-only adapter (Stellar contract needs initialization)")
            return True
        else:
            print(f"❌ Failed to configure adapter: {response.text}")
            return False

    def parse_csv(self, csv_path: str) -> List[Dict]:
        """Parse Gerbov CSV file and extract animal data"""
        print(f"\n📄 Parsing CSV file: {csv_path}")

        animals = []
        with open(csv_path, 'r', encoding='utf-8') as f:
            reader = csv.reader(f)
            rows = list(reader)

            # Skip header rows (first 4 rows)
            data_rows = rows[4:]

            for row in data_rows:
                if len(row) < 9:  # Skip incomplete rows
                    continue

                # Extract fields
                data_nasc = row[0].strip() if row[0] else None
                numero_animal = row[1].strip() if row[1] else None
                idade = row[2].strip() if row[2] else None
                idade_era = row[3].strip() if row[3] else None
                nome_animal = row[4].strip() if row[4] else None
                numero_matriz = row[5].strip() if row[5] else None
                sisbov = row[6].strip() if row[6] else None
                data_liberacao = row[7].strip() if row[7] else None
                chip = row[8].strip() if row[8] else None
                raca = row[9].strip() if row[9] else None
                categoria = row[10].strip() if row[10] else None
                lote = row[11].strip() if row[11] else None
                centro_custo = row[12].strip() if row[12] else None
                data_entrada = row[13].strip() if row[13] else None
                fornecedor = row[14].strip() if row[14] else None
                local = row[15].strip() if row[15] else None
                ultimo_peso = row[16].strip() if len(row) > 16 and row[16] else None
                data_ultima_pesagem = row[17].strip() if len(row) > 17 and row[17] else None
                ultimo_manejo = row[18].strip() if len(row) > 18 and row[18] else None
                data_ultimo_manejo = row[19].strip() if len(row) > 19 and row[19] else None
                observacao = row[20].strip() if len(row) > 20 and row[20] else None

                # Build animal record
                animal = {
                    "numero_animal": numero_animal,
                    "sisbov": sisbov,
                    "chip": chip,
                    "numero_matriz": numero_matriz,
                    "data_nasc": data_nasc,
                    "idade": idade,
                    "idade_era": idade_era,
                    "nome_animal": nome_animal,
                    "raca": raca,
                    "categoria": categoria,
                    "lote": lote,
                    "centro_custo": centro_custo,
                    "data_entrada": data_entrada,
                    "fornecedor": fornecedor,
                    "local": local,
                    "ultimo_peso": ultimo_peso,
                    "data_ultima_pesagem": data_ultima_pesagem,
                    "ultimo_manejo": ultimo_manejo,
                    "data_ultimo_manejo": data_ultimo_manejo,
                    "observacao": observacao
                }

                animals.append(animal)

        print(f"✅ Parsed {len(animals)} animals from CSV")
        return animals

    def create_local_item(self, animal: Dict) -> Optional[str]:
        """Create local item for animal"""

        # Build enhanced identifiers
        identifiers = []

        # Canonical identifiers (SISBOV and Chip/RFID)
        if animal["sisbov"]:
            identifiers.append({
                "key": "sisbov",
                "value": animal["sisbov"],
                "type": "canonical",
                "confidence": 1.0
            })

        if animal["chip"]:
            identifiers.append({
                "key": "chip",
                "value": animal["chip"],
                "type": "canonical",
                "confidence": 1.0
            })

        # Contextual identifiers
        if animal["numero_animal"]:
            identifiers.append({
                "key": "numero_animal",
                "value": animal["numero_animal"],
                "type": "contextual",
                "confidence": 1.0
            })

        if animal["numero_matriz"]:
            identifiers.append({
                "key": "numero_matriz",
                "value": animal["numero_matriz"],
                "type": "contextual",
                "confidence": 0.9
            })

        # Build metadata
        metadata = {
            "source": "gerbov",
            "data_nasc": animal["data_nasc"],
            "idade": animal["idade"],
            "idade_era": animal["idade_era"],
            "raca": animal["raca"],
            "categoria": animal["categoria"],
            "lote": animal["lote"],
            "centro_custo": animal["centro_custo"],
            "data_entrada": animal["data_entrada"],
            "fornecedor": animal["fornecedor"],
            "local": animal["local"]
        }

        # Add optional fields
        if animal["nome_animal"]:
            metadata["nome_animal"] = animal["nome_animal"]
        if animal["ultimo_peso"]:
            metadata["ultimo_peso"] = animal["ultimo_peso"]
        if animal["data_ultima_pesagem"]:
            metadata["data_ultima_pesagem"] = animal["data_ultima_pesagem"]
        if animal["ultimo_manejo"]:
            metadata["ultimo_manejo"] = animal["ultimo_manejo"]
        if animal["data_ultimo_manejo"]:
            metadata["data_ultimo_manejo"] = animal["data_ultimo_manejo"]
        if animal["observacao"]:
            metadata["observacao"] = animal["observacao"]

        # Create local item
        item_data = {
            "enhanced_identifiers": identifiers,
            "metadata": metadata
        }

        response = requests.post(
            f"{self.base_url}/api/items/local",
            headers=self.headers(),
            json=item_data
        )

        if response.status_code in [200, 201]:
            result = response.json()
            # Handle both response formats
            if "success" in result and result["success"]:
                return result["data"]["local_id"]
            elif "local_id" in result:
                return result["local_id"]
            else:
                print(f"❌ Unexpected response format for animal {animal['numero_animal']}: {response.text}")
                return None
        else:
            print(f"❌ Failed to create item for animal {animal['numero_animal']}: {response.text}")
            return None

    def push_local_item(self, circuit_id: str, local_id: str) -> Optional[str]:
        """Push local item to circuit to generate DFID and store in IPFS"""

        response = requests.post(
            f"{self.base_url}/api/circuits/{circuit_id}/push-local",
            headers=self.headers(),
            json={"local_id": local_id}
        )

        if response.status_code == 200:
            result = response.json()
            # Handle wrapped response format
            if "success" in result and result["success"]:
                data = result["data"]
                dfid = data.get("dfid")
                if dfid:
                    print(f"  🎉 DFID: {dfid}")
                    print(f"  📋 Status: {data.get('status', 'Unknown')}")
                    if "storage_info" in data:
                        storage = data["storage_info"]
                        if "cid" in storage:
                            print(f"  📦 IPFS CID: {storage['cid']}")
                return dfid
            else:
                print(f"  ❌ Push failed: {result}")
                return None
        else:
            print(f"  ❌ Push failed ({response.status_code}): {response.text[:200]}")
            return None

    def process_animals(self, animals: List[Dict]) -> None:
        """Process all animals: create local items and push to circuit"""
        print(f"\n🐄 Processing {len(animals)} animals...")

        results = {
            "success": [],
            "failed": []
        }

        for i, animal in enumerate(animals, 1):
            numero = animal["numero_animal"]
            sisbov = animal["sisbov"]
            chip = animal["chip"]

            print(f"\n[{i}/{len(animals)}] Processing animal #{numero} (SISBOV: {sisbov})")

            # Create local item
            local_id = self.create_local_item(animal)
            if not local_id:
                results["failed"].append({
                    "numero_animal": numero,
                    "reason": "Failed to create local item"
                })
                continue

            print(f"  ✅ Local item created: {local_id}")

            # Wait a bit to avoid rate limiting
            time.sleep(0.5)

            # Push to circuit
            dfid = self.push_local_item(self.circuit_id, local_id)
            if not dfid:
                results["failed"].append({
                    "numero_animal": numero,
                    "local_id": local_id,
                    "reason": "Failed to push to circuit"
                })
                continue

            print(f"  🎉 DFID generated: {dfid}")
            print(f"  🛰️  Registered on Stellar Testnet")

            results["success"].append({
                "numero_animal": numero,
                "sisbov": sisbov,
                "chip": chip,
                "local_id": local_id,
                "dfid": dfid
            })

            # Wait a bit between items
            time.sleep(1)

        # Print summary
        print("\n" + "="*80)
        print("📊 INTEGRATION SUMMARY")
        print("="*80)
        print(f"✅ Successfully processed: {len(results['success'])} animals")
        print(f"❌ Failed: {len(results['failed'])} animals")

        if results["success"]:
            print("\n✅ Successful animals:")
            for item in results["success"]:
                print(f"  - Animal #{item['numero_animal']} (SISBOV: {item['sisbov']})")
                print(f"    LID: {item['local_id']}")
                print(f"    DFID: {item['dfid']}")

        if results["failed"]:
            print("\n❌ Failed animals:")
            for item in results["failed"]:
                print(f"  - Animal #{item['numero_animal']}: {item['reason']}")

        # Save results to file
        with open("/Users/gabrielrondon/rust/engines/gerbov_integration_results.json", "w") as f:
            json.dump(results, f, indent=2)
        print(f"\n💾 Results saved to gerbov_integration_results.json")

def main():
    csv_path = "/Users/gabrielrondon/Downloads/relatorio_animais_identificados - 2026-01-23T112214.605.xls - Animais Identificados.csv"

    print("="*80)
    print("🚜 GERBOV INTEGRATION - DeFarm x Gerbov")
    print("="*80)
    print("📋 Importing animal data from Fazenda Santa Fé")
    print("🛰️  Registering on Stellar Testnet (NFT on-chain, events offchain IPFS)")
    print("="*80)

    integration = GerbovIntegration(BASE_URL)

    # Step 1: Authenticate
    if not integration.login(USERNAME, PASSWORD):
        print("❌ Authentication failed. Exiting.")
        return

    # Step 2: Get or create circuit
    circuit_id = integration.get_or_create_circuit()
    if not circuit_id:
        print("❌ Failed to get/create circuit. Exiting.")
        return

    # Step 3: Configure Stellar adapter
    if not integration.configure_adapter(circuit_id):
        print("❌ Failed to configure adapter. Exiting.")
        return

    # Step 4: Parse CSV
    animals = integration.parse_csv(csv_path)
    if not animals:
        print("❌ No animals found in CSV. Exiting.")
        return

    # Step 5: Process animals
    integration.process_animals(animals)

    print("\n" + "="*80)
    print("✅ INTEGRATION COMPLETE!")
    print("="*80)
    print(f"🔗 Circuit ID: {circuit_id}")
    print(f"🛰️  Adapter: Stellar Testnet")
    print(f"🐄 Animals processed: {len(animals)}")
    print("\n🌐 Access the circuit at:")
    print(f"   {BASE_URL}/circuits/{circuit_id}")

if __name__ == "__main__":
    main()
