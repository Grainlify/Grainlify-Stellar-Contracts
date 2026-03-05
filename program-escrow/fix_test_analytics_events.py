import re

with open("src/test_analytics_events.rs", "r") as f:
    text = f.read()

# 1. find_event_by_topic
text = text.replace("let topics = event.0;", "let topics = event.1;")
text = text.replace("return Some((topics, event.1));", "return Some((topics, event.2));")

# 2. Extract Fields (Manual Regex without DOTALL overlapping)
text = text.replace("let amount = i128::try_from_val(&env, &data_map.get(Symbol::new(&env, \"amount\")).unwrap()).unwrap();", 
                    "let value_val = data_map.get(Symbol::new(&env, \"amount\")).unwrap();\n    let amount = <i128 as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::try_from_val(&env, &value_val).unwrap();")

text = text.replace("let threshold = i128::try_from_val(&env, &data_map.get(Symbol::new(&env, \"threshold\")).unwrap()).unwrap();", 
                    "let value_val = data_map.get(Symbol::new(&env, \"threshold\")).unwrap();\n    let threshold = <i128 as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::try_from_val(&env, &value_val).unwrap();")

text = text.replace("let total_funds = i128::try_from_val(&env, &data_map.get(Symbol::new(&env, \"total_funds\")).unwrap()).unwrap();",
                    "let value_val = data_map.get(Symbol::new(&env, \"total_funds\")).unwrap();\n    let total_funds = <i128 as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::try_from_val(&env, &value_val).unwrap();")

text = text.replace("let remaining_balance = i128::try_from_val(&env, &data_map.get(Symbol::new(&env, \"remaining_balance\")).unwrap()).unwrap();",
                    "let value_val = data_map.get(Symbol::new(&env, \"remaining_balance\")).unwrap();\n    let remaining_balance = <i128 as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::try_from_val(&env, &value_val).unwrap();")

text = text.replace("let total_paid_out = i128::try_from_val(&env, &data_map.get(Symbol::new(&env, \"total_paid_out\")).unwrap()).unwrap();",
                    "let value_val = data_map.get(Symbol::new(&env, \"total_paid_out\")).unwrap();\n    let total_paid_out = <i128 as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::try_from_val(&env, &value_val).unwrap();")

text = text.replace("let payout_count = u32::try_from_val(&env, &data_map.get(Symbol::new(&env, \"payout_count\")).unwrap()).unwrap();",
                    "let value_val = data_map.get(Symbol::new(&env, \"payout_count\")).unwrap();\n    let payout_count = <u32 as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::try_from_val(&env, &value_val).unwrap();")

text = text.replace("let schedule_id = u64::try_from_val(&env, &data_map.get(Symbol::new(&env, \"schedule_id\")).unwrap()).unwrap();",
                    "let value_val = data_map.get(Symbol::new(&env, \"schedule_id\")).unwrap();\n    let schedule_id = <u64 as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::try_from_val(&env, &value_val).unwrap();")

text = text.replace("let scheduled_count = u32::try_from_val(&env, &data_map.get(Symbol::new(&env, \"scheduled_count\")).unwrap()).unwrap();",
                    "let value_val = data_map.get(Symbol::new(&env, \"scheduled_count\")).unwrap();\n    let scheduled_count = <u32 as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::try_from_val(&env, &value_val).unwrap();")

# And the Map extraction
text = text.replace("let data_map: Map<Symbol, Val> = Map::try_from_val(&env, &data).unwrap();", 
                    "let data_map: Map<Symbol, Val> = soroban_sdk::TryFromVal::try_from_val(&env, &data).unwrap();")
text = text.replace("let data_map: Map<Symbol, Val> = Map::try_from_val(&env, &event.1).unwrap();", 
                    "let data_map: Map<Symbol, Val> = soroban_sdk::TryFromVal::try_from_val(&env, &event.2).unwrap();")


# Handle event compactness parsing
compactness = r"""
    // Verify all events have compact payloads (only necessary fields)
    let events = env.events().all();
    for i in 0..events.len() {
        let event = events.get(i).unwrap();
        let data = event.2;
        
        let data_map: Map<Symbol, Val> = soroban_sdk::TryFromVal::try_from_val(&env, &data).unwrap();
        // All event payloads should be maps with version field
        assert!(data_map.contains_key(Symbol::new(&env, "version")));
        
        // Verify field count is reasonable (not bloated)
        assert!(data_map.len() <= 10, "Event payload should be compact");
    }
}
"""
text = re.sub(r'let events = env\.events\(\)\.all\(\);\s*for i in 0\.\.events\.len\(\).*?compact"\);\s*\}\s*\}', compactness, text, flags=re.DOTALL)


# Fix register_stellar_asset_contract -> register_stellar_asset_contract_v2
text = text.replace("env.register_stellar_asset_contract(token_admin.clone())", "env.register_stellar_asset_contract_v2(token_admin.clone()).address()")

with open("src/test_analytics_events.rs", "w") as f:
    f.write(text)
