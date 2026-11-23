// Source d'alimentation : secteur ou batterie.

#[repr(C)]
struct SystemPowerStatus {
    ac_line_status: u8,
    battery_flag: u8,
    battery_life_percent: u8,
    system_status_flag: u8,
    battery_life_time: u32,
    battery_full_life_time: u32,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetSystemPowerStatus(status: *mut SystemPowerStatus) -> i32;
}

fn parse_ac(line: u8) -> Option<bool> {
    match line {
        0 => Some(false),
        1 => Some(true),
        _ => None, // 255 : inconnu
    }
}

/// Vrai sur secteur, faux sur batterie, `None` si Windows ne le sait pas.
/// Un PC fixe sans batterie se déclare toujours sur secteur.
pub fn on_ac() -> Option<bool> {
    let mut s = SystemPowerStatus {
        ac_line_status: 255,
        battery_flag: 0,
        battery_life_percent: 0,
        system_status_flag: 0,
        battery_life_time: 0,
        battery_full_life_time: 0,
    };
    if unsafe { GetSystemPowerStatus(&mut s) } == 0 {
        return None;
    }
    parse_ac(s.ac_line_status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ac_line_status() {
        assert_eq!(parse_ac(1), Some(true));
        assert_eq!(parse_ac(0), Some(false));
        assert_eq!(parse_ac(255), None);
    }

    #[test]
    fn status_struct_size() {
        assert_eq!(std::mem::size_of::<SystemPowerStatus>(), 12);
    }
}
