import re
c = open("src/test.rs").read()

old_assert = """fn assert_event_data_has_v2_tag(env: &Env, data: &Val) {
    if let Ok(data_map) = Map::<Symbol, Val>::try_from_val(env, data) {
        if data_map.contains_key(Symbol::new(env, "duration")) || data_map.contains_key(Symbol::new(env, "caller")) || data_map.contains_key(Symbol::new(env, "lock")) {
            return; // Skip metric/op/pause events
        }
        let version_val = data_map
            .get(Symbol::new(env, "version"))
            .unwrap_or_else(|| panic!("event payload must contain version field"));
    let version = u32::try_from_val(env, &version_val).expect("version should decode as u32");
    assert_eq!(version, 2);
}"""

new_assert = """fn assert_event_data_has_v2_tag(env: &Env, data: &Val) {
    if let Ok(data_map) = Map::<Symbol, Val>::try_from_val(env, data) {
        if data_map.contains_key(Symbol::new(env, "duration")) || data_map.contains_key(Symbol::new(env, "caller")) || data_map.contains_key(Symbol::new(env, "lock")) {
            return; // Skip metric/op/pause events
        }
        let version_val = data_map
            .get(Symbol::new(env, "version"))
            .unwrap_or_else(|| panic!("event payload must contain version field"));
        let version = u32::try_from_val(env, &version_val).expect("version should decode as u32");
        assert_eq!(version, 2);
    }
}"""

c = c.replace(old_assert, new_assert)
open("src/test.rs", "w").write(c)
