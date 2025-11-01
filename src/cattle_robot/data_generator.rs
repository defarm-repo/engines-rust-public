use chrono::NaiveDate;
use rand::distributions::{Distribution, WeightedIndex};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::hash_generator::HashGenerator;

/// Brazilian cattle breeds with realistic distribution
const BREEDS: &[(&str, u32)] = &[
    ("Nelore", 50),   // Most common in Brazil
    ("Angus", 15),    // Popular for meat
    ("Brahman", 12),  // Heat resistant
    ("Senepol", 8),   // Adaptable
    ("Simmental", 5), // Dual purpose
    ("Hereford", 4),  // Beef cattle
    ("Canchim", 3),   // Brazilian breed
    ("Caracu", 2),    // Brazilian native
    ("Guzerá", 1),    // Dairy/beef
];

/// Brazilian states with realistic distribution
const STATES: &[(&str, u32)] = &[
    ("MS", 60), // Mato Grosso do Sul (main focus)
    ("MT", 15), // Mato Grosso
    ("SP", 10), // São Paulo
    ("GO", 10), // Goiás
    ("RS", 5),  // Rio Grande do Sul
];

/// Municipalities with IBGE codes (sample for each state)
const MUNICIPALITIES: &[(&str, &str, &str)] = &[
    // MS - Mato Grosso do Sul
    ("MS", "5002704", "Campo Grande"),
    ("MS", "5003702", "Dourados"),
    ("MS", "5008305", "Três Lagoas"),
    ("MS", "5004106", "Corumbá"),
    ("MS", "5006200", "Ponta Porã"),
    ("MS", "5001102", "Aquidauana"),
    ("MS", "5004304", "Coxim"),
    ("MS", "5006903", "Rio Brilhante"),
    ("MS", "5007406", "São Gabriel do Oeste"),
    ("MS", "5000203", "Água Clara"),
    // MT - Mato Grosso
    ("MT", "5103403", "Cuiabá"),
    ("MT", "5107909", "Rondonópolis"),
    ("MT", "5103908", "Diamantino"),
    ("MT", "5108402", "Sinop"),
    ("MT", "5100201", "Água Boa"),
    // SP - São Paulo
    ("SP", "3501608", "Andradina"),
    ("SP", "3503208", "Araçatuba"),
    ("SP", "3549904", "São José do Rio Preto"),
    ("SP", "3502804", "Barretos"),
    // GO - Goiás
    ("GO", "5208707", "Goiânia"),
    ("GO", "5200258", "Anápolis"),
    ("GO", "5221858", "Rio Verde"),
    ("GO", "5212501", "Jataí"),
    // RS - Rio Grande do Sul
    ("RS", "4314902", "Porto Alegre"),
    ("RS", "4301602", "Bagé"),
    ("RS", "4303905", "Cachoeira do Sul"),
];

/// Generic farm/ranch names (will be hashed)
const FARM_NAMES: &[&str] = &[
    "Fazenda Santa Maria",
    "Fazenda São José",
    "Fazenda Três Lagoas",
    "Fazenda Boa Vista",
    "Fazenda Esperança",
    "Fazenda Progresso",
    "Fazenda União",
    "Fazenda Harmonia",
    "Fazenda Primavera",
    "Fazenda Bandeirantes",
    "Rancho Grande",
    "Rancho Verde",
    "Sítio Alegre",
    "Estância Nova",
    "Estância Real",
    "Hacienda do Sul",
    "Agropecuária Central",
    "Agropecuária Modelo",
    "Pecuária União",
    "Pecuária Progresso",
];

/// Generic company names (cooperatives, slaughterhouses)
const COMPANY_NAMES: &[&str] = &[
    "Cooperativa Agrícola MS",
    "Cooperativa Pecuarista Central",
    "Frigorífico União",
    "Frigorífico Modelo",
    "Agropecuária Integrada",
    "Pecuária e Comércio Sul",
    "Cooperativa Regional",
    "Frigorífico Regional",
    "Indústria de Carnes Central",
    "Agronegócio Integrado",
];

/// Generic veterinarian names (will be hashed)
const VET_NAMES: &[&str] = &[
    "Dr. João Silva",
    "Dra. Maria Santos",
    "Dr. Pedro Oliveira",
    "Dra. Ana Costa",
    "Dr. Carlos Souza",
    "Dra. Juliana Almeida",
    "Dr. Roberto Lima",
    "Dra. Fernanda Rodrigues",
    "Dr. Marcos Ferreira",
    "Dra. Patricia Gomes",
    "Dr. André Martins",
    "Dra. Luciana Barbosa",
    "Dr. Ricardo Pereira",
    "Dra. Camila Ribeiro",
    "Dr. Felipe Araújo",
    "Dra. Beatriz Carvalho",
    "Dr. Lucas Dias",
    "Dra. Amanda Moreira",
    "Dr. Thiago Cunha",
    "Dra. Renata Teixeira",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CattleData {
    pub sisbov: String,
    pub birth_date: NaiveDate,
    pub breed: String,
    pub gender: String,
    pub state: String,
    pub municipality_code: String,
    pub municipality_name: String,
    pub owner_hash: String,
    pub owner_type: String, // farm, company, cooperative
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    pub event_type: String,
    pub event_date: NaiveDate,
    pub from_owner_hash: Option<String>,
    pub to_owner_hash: Option<String>,
    pub vet_hash: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct DataGenerator {
    rng: ThreadRng,
    used_sisbov: Vec<String>,
    owner_pool: Vec<(String, String)>, // (hash, type)
    vet_pool: Vec<String>,
}

impl Default for DataGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl DataGenerator {
    pub fn new() -> Self {
        let rng = thread_rng();

        // Pre-generate owner hashes
        let mut owner_pool = Vec::new();
        for farm_name in FARM_NAMES {
            owner_pool.push((HashGenerator::hash_farm(farm_name), "farm".to_string()));
        }
        for company_name in COMPANY_NAMES {
            owner_pool.push((
                HashGenerator::hash_company(company_name),
                "company".to_string(),
            ));
        }

        // Pre-generate vet hashes
        let vet_pool: Vec<String> = VET_NAMES
            .iter()
            .map(|name| HashGenerator::hash_vet(name))
            .collect();

        Self {
            rng,
            used_sisbov: Vec::new(),
            owner_pool,
            vet_pool,
        }
    }

    /// Generate a unique SISBOV number (BR + 12 digits)
    pub fn generate_sisbov(&mut self) -> String {
        loop {
            // Generate realistic SISBOV: BR + 12 random digits
            // Use range that looks realistic but is clearly synthetic
            let num = self.rng.gen_range(100_000_000_000u64..999_999_999_999u64);
            let sisbov = format!("BR{num}");

            if !self.used_sisbov.contains(&sisbov) {
                self.used_sisbov.push(sisbov.clone());
                return sisbov;
            }
        }
    }

    /// Generate cattle data with realistic distributions
    pub fn generate_cattle(&mut self) -> CattleData {
        // Select breed with weighted distribution
        let breed_weights: Vec<u32> = BREEDS.iter().map(|(_, w)| *w).collect();
        let breed_dist = WeightedIndex::new(&breed_weights).unwrap();
        let breed = BREEDS[breed_dist.sample(&mut self.rng)].0.to_string();

        // Select state with weighted distribution
        let state_weights: Vec<u32> = STATES.iter().map(|(_, w)| *w).collect();
        let state_dist = WeightedIndex::new(&state_weights).unwrap();
        let state = STATES[state_dist.sample(&mut self.rng)].0.to_string();

        // Select municipality from the chosen state
        let state_municipalities: Vec<_> = MUNICIPALITIES
            .iter()
            .filter(|(s, _, _)| *s == state)
            .collect();
        let (_, municipality_code, municipality_name) =
            state_municipalities.choose(&mut self.rng).unwrap();

        // Generate birth date (2020-2024, with seasonal peaks in Sep-Nov)
        let birth_date = self.generate_birth_date();

        // Generate gender (roughly 50/50)
        let gender = if self.rng.gen_bool(0.5) {
            "Male"
        } else {
            "Female"
        }
        .to_string();

        // Select random owner from pool
        let (owner_hash, owner_type) = self.owner_pool.choose(&mut self.rng).unwrap().clone();

        CattleData {
            sisbov: self.generate_sisbov(),
            birth_date,
            breed,
            gender,
            state,
            municipality_code: municipality_code.to_string(),
            municipality_name: municipality_name.to_string(),
            owner_hash,
            owner_type,
        }
    }

    /// Generate realistic birth date with seasonal peaks
    fn generate_birth_date(&mut self) -> NaiveDate {
        let year = self.rng.gen_range(2020..=2024);

        // Weighted months: Sep-Nov are peak calving season in Brazil
        let month_weights = [5, 5, 5, 5, 5, 5, 5, 8, 12, 12, 10, 5]; // Jan-Dec
        let month_dist = WeightedIndex::new(month_weights).unwrap();
        let month = month_dist.sample(&mut self.rng) + 1;

        let max_day = match month {
            2 => {
                if year % 4 == 0 {
                    29
                } else {
                    28
                }
            }
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        let day = self.rng.gen_range(1..=max_day);

        NaiveDate::from_ymd_opt(year, month as u32, day).unwrap()
    }

    /// Generate birth event for new cattle
    pub fn generate_birth_event(&mut self, cattle: &CattleData) -> EventData {
        let mut metadata = HashMap::new();
        metadata.insert(
            "birth_weight_kg".to_string(),
            serde_json::json!(self.rng.gen_range(25..40)),
        );
        metadata.insert(
            "mother_sisbov".to_string(),
            serde_json::json!(format!(
                "BR{}",
                self.rng.gen_range(100_000_000_000u64..999_999_999_999u64)
            )),
        );
        metadata.insert(
            "location".to_string(),
            serde_json::json!(cattle.municipality_name.clone()),
        );

        EventData {
            event_type: "birth".to_string(),
            event_date: cattle.birth_date,
            from_owner_hash: None,
            to_owner_hash: Some(cattle.owner_hash.clone()),
            vet_hash: None,
            metadata,
        }
    }

    /// Generate weight measurement event
    pub fn generate_weight_event(
        &mut self,
        cattle: &CattleData,
        current_date: NaiveDate,
    ) -> EventData {
        let age_days = (current_date - cattle.birth_date).num_days();

        // Realistic weight based on age (roughly 0.7-1.0 kg/day gain)
        let birth_weight = 30.0;
        let daily_gain = self.rng.gen_range(0.7..1.0);
        let weight = birth_weight + (age_days as f64 * daily_gain);

        let mut metadata = HashMap::new();
        metadata.insert("weight_kg".to_string(), serde_json::json!(weight as u32));
        metadata.insert("age_days".to_string(), serde_json::json!(age_days));

        EventData {
            event_type: "weight".to_string(),
            event_date: current_date,
            from_owner_hash: None,
            to_owner_hash: None,
            vet_hash: None,
            metadata,
        }
    }

    /// Generate ownership transfer event
    pub fn generate_transfer_event(
        &mut self,
        cattle: &CattleData,
        transfer_date: NaiveDate,
    ) -> EventData {
        let (new_owner_hash, new_owner_type) =
            self.owner_pool.choose(&mut self.rng).unwrap().clone();

        let mut metadata = HashMap::new();
        metadata.insert("transfer_type".to_string(), serde_json::json!("sale"));
        metadata.insert(
            "new_owner_type".to_string(),
            serde_json::json!(new_owner_type),
        );

        EventData {
            event_type: "transfer".to_string(),
            event_date: transfer_date,
            from_owner_hash: Some(cattle.owner_hash.clone()),
            to_owner_hash: Some(new_owner_hash),
            vet_hash: None,
            metadata,
        }
    }

    /// Generate vaccination event
    pub fn generate_vaccination_event(
        &mut self,
        cattle: &CattleData,
        vacc_date: NaiveDate,
    ) -> EventData {
        let vaccines = ["FMD", "Brucellosis", "Clostridial", "Respiratory"];
        let vaccine = vaccines.choose(&mut self.rng).unwrap();

        let vet_hash = self.vet_pool.choose(&mut self.rng).unwrap().clone();

        let mut metadata = HashMap::new();
        metadata.insert("vaccine_type".to_string(), serde_json::json!(vaccine));
        metadata.insert(
            "batch".to_string(),
            serde_json::json!(format!("VAC{}", self.rng.gen_range(1000..9999))),
        );

        EventData {
            event_type: "vaccination".to_string(),
            event_date: vacc_date,
            from_owner_hash: None,
            to_owner_hash: None,
            vet_hash: Some(vet_hash),
            metadata,
        }
    }

    /// Generate movement event (between farms/locations)
    pub fn generate_movement_event(
        &mut self,
        cattle: &CattleData,
        move_date: NaiveDate,
    ) -> EventData {
        // Select a different municipality in same state
        let state_municipalities: Vec<_> = MUNICIPALITIES
            .iter()
            .filter(|(s, _, _)| *s == cattle.state)
            .collect();
        let (_, _, destination) = state_municipalities.choose(&mut self.rng).unwrap();

        let mut metadata = HashMap::new();
        metadata.insert(
            "from_location".to_string(),
            serde_json::json!(cattle.municipality_name.clone()),
        );
        metadata.insert(
            "to_location".to_string(),
            serde_json::json!(destination.to_string()),
        );
        metadata.insert("reason".to_string(), serde_json::json!("pasture_rotation"));

        EventData {
            event_type: "movement".to_string(),
            event_date: move_date,
            from_owner_hash: None,
            to_owner_hash: None,
            vet_hash: None,
            metadata,
        }
    }

    /// Select random event type with realistic distribution
    pub fn select_event_type(&mut self) -> &'static str {
        let event_weights = [40, 30, 20, 5, 5]; // birth, weight, transfer, vaccination, movement
        let events = ["birth", "weight", "transfer", "vaccination", "movement"];
        let event_dist = WeightedIndex::new(event_weights).unwrap();
        events[event_dist.sample(&mut self.rng)]
    }

    /// Get random owner hash from pool
    pub fn get_random_owner(&mut self) -> (String, String) {
        self.owner_pool.choose(&mut self.rng).unwrap().clone()
    }

    /// Get random vet hash from pool
    pub fn get_random_vet(&mut self) -> String {
        self.vet_pool.choose(&mut self.rng).unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sisbov_generation() {
        let mut gen = DataGenerator::new();
        let sisbov = gen.generate_sisbov();
        assert!(sisbov.starts_with("BR"));
        assert_eq!(sisbov.len(), 14); // BR + 12 digits
    }

    #[test]
    fn test_cattle_generation() {
        let mut gen = DataGenerator::new();
        let cattle = gen.generate_cattle();
        assert!(cattle.sisbov.starts_with("BR"));
        assert!(!cattle.breed.is_empty());
        assert!(!cattle.owner_hash.is_empty());
        assert!(cattle.owner_hash.starts_with("hash:"));
    }

    #[test]
    fn test_event_generation() {
        let mut gen = DataGenerator::new();
        let cattle = gen.generate_cattle();

        let birth_event = gen.generate_birth_event(&cattle);
        assert_eq!(birth_event.event_type, "birth");
        assert!(birth_event.metadata.contains_key("birth_weight_kg"));

        let weight_event =
            gen.generate_weight_event(&cattle, NaiveDate::from_ymd_opt(2024, 6, 1).unwrap());
        assert_eq!(weight_event.event_type, "weight");
        assert!(weight_event.metadata.contains_key("weight_kg"));
    }

    #[test]
    fn test_unique_sisbov() {
        let mut gen = DataGenerator::new();
        let mut sisbovs = std::collections::HashSet::new();

        for _ in 0..100 {
            let sisbov = gen.generate_sisbov();
            assert!(!sisbovs.contains(&sisbov), "Duplicate SISBOV generated");
            sisbovs.insert(sisbov);
        }
    }
}
