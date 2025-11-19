#![allow(non_snake_case)]
#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, log, symbol_short, Address, Env, String, Symbol,
};

// Structure to store carbon footprint data for entities
#[contracttype]
#[derive(Clone)]
pub struct CarbonRecord {
    pub entity_id: u64,
    pub entity_name: String,
    pub entity_type: String,   // "Company" or "Product"
    pub carbon_emission: i128, // in kg CO2
    pub verification_status: bool,
    pub timestamp: u64,
}

// Platform-wide statistics
#[contracttype]
#[derive(Clone)]
pub struct PlatformStats {
    pub total_records: u64,
    pub verified_records: u64,
    pub total_emissions_tracked: i128,
    pub company_count: u64,
    pub product_count: u64,
}

// Storage keys
const RECORD_COUNT: Symbol = symbol_short!("REC_CNT");
const STATS: Symbol = symbol_short!("STATS");

// Mapping entity_id to CarbonRecord
#[contracttype]
pub enum RecordBook {
    Record(u64),
}

#[contract]
pub struct GreenScoreLedger;

#[contractimpl]
impl GreenScoreLedger {
    // Register a new carbon footprint record
    pub fn register_carbon_record(
        env: Env,
        submitter: Address,
        entity_name: String,
        entity_type: String,
        carbon_emission: i128,
    ) -> u64 {
        // Verify the submitter is calling this function
        submitter.require_auth();

        // Validate inputs
        if carbon_emission < 0 {
            panic!("Carbon emission cannot be negative");
        }

        // Validate entity type - compare String objects directly
        let company_type = String::from_str(&env, "Company");
        let product_type = String::from_str(&env, "Product");

        if entity_type != company_type && entity_type != product_type {
            panic!("Entity type must be 'Company' or 'Product'");
        }

        // Get and increment record counter
        let mut record_count: u64 = env.storage().instance().get(&RECORD_COUNT).unwrap_or(0);
        record_count += 1;

        // Get current timestamp
        let time = env.ledger().timestamp();

        // Create carbon record
        let carbon_record = CarbonRecord {
            entity_id: record_count,
            entity_name,
            entity_type: entity_type.clone(),
            carbon_emission,
            verification_status: false, // Initially unverified
            timestamp: time,
        };

        // Update platform statistics
        let mut stats = Self::get_platform_stats(env.clone());
        stats.total_records += 1;
        stats.total_emissions_tracked += carbon_emission;

        // Update entity count based on type
        if entity_type == company_type {
            stats.company_count += 1;
        } else {
            stats.product_count += 1;
        }

        // Store data
        env.storage()
            .instance()
            .set(&RecordBook::Record(record_count), &carbon_record);
        env.storage().instance().set(&STATS, &stats);
        env.storage().instance().set(&RECORD_COUNT, &record_count);
        env.storage().instance().extend_ttl(5000, 5000);

        log!(&env, "Carbon record created with ID: {}", record_count);
        record_count
    }

    // Verify a carbon footprint record (by authorized verifier)
    pub fn verify_carbon_record(env: Env, verifier: Address, entity_id: u64) {
        // Verify the verifier is calling this function
        verifier.require_auth();

        // Get carbon record
        let mut carbon_record = Self::get_carbon_record(env.clone(), entity_id);

        if carbon_record.entity_id == 0 {
            panic!("Carbon record not found");
        }

        if carbon_record.verification_status {
            panic!("Record is already verified");
        }

        // Mark as verified
        carbon_record.verification_status = true;

        // Update statistics
        let mut stats = Self::get_platform_stats(env.clone());
        stats.verified_records += 1;

        // Store updated data
        env.storage()
            .instance()
            .set(&RecordBook::Record(entity_id), &carbon_record);
        env.storage().instance().set(&STATS, &stats);
        env.storage().instance().extend_ttl(5000, 5000);

        log!(&env, "Carbon record ID: {} has been verified", entity_id);
    }

    // Update carbon emission data for an existing record
    pub fn update_carbon_emission(env: Env, updater: Address, entity_id: u64, new_emission: i128) {
        // Verify the updater is calling this function
        updater.require_auth();

        // Validate new emission value
        if new_emission < 0 {
            panic!("Carbon emission cannot be negative");
        }

        // Get carbon record
        let mut carbon_record = Self::get_carbon_record(env.clone(), entity_id);

        if carbon_record.entity_id == 0 {
            panic!("Carbon record not found");
        }

        // Calculate emission difference
        let emission_difference = new_emission - carbon_record.carbon_emission;

        // Store original verification status before update
        let was_verified = carbon_record.verification_status;

        // Update record
        carbon_record.carbon_emission = new_emission;
        carbon_record.timestamp = env.ledger().timestamp();
        carbon_record.verification_status = false; // Reset verification after update

        // Update platform statistics
        let mut stats = Self::get_platform_stats(env.clone());
        stats.total_emissions_tracked += emission_difference;

        // If record was verified, decrement verified count
        if was_verified {
            stats.verified_records -= 1;
        }

        // Store updated data
        env.storage()
            .instance()
            .set(&RecordBook::Record(entity_id), &carbon_record);
        env.storage().instance().set(&STATS, &stats);
        env.storage().instance().extend_ttl(5000, 5000);

        log!(&env, "Carbon record ID: {} has been updated", entity_id);
    }

    // View carbon record details by entity ID
    pub fn get_carbon_record(env: Env, entity_id: u64) -> CarbonRecord {
        let key = RecordBook::Record(entity_id);

        env.storage().instance().get(&key).unwrap_or(CarbonRecord {
            entity_id: 0,
            entity_name: String::from_str(&env, "Not_Found"),
            entity_type: String::from_str(&env, "Not_Found"),
            carbon_emission: 0,
            verification_status: false,
            timestamp: 0,
        })
    }

    // View platform statistics
    pub fn get_platform_stats(env: Env) -> PlatformStats {
        env.storage()
            .instance()
            .get(&STATS)
            .unwrap_or(PlatformStats {
                total_records: 0,
                verified_records: 0,
                total_emissions_tracked: 0,
                company_count: 0,
                product_count: 0,
            })
    }
}
