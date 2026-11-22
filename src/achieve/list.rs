//! The 56 achievements (`.claude/docs/achievements.md`). `key` is permanent: it names the host's id and the line in
//! `achievements.txt`, and Play Games can't delete a published one.

pub struct Ach {
    pub key: &'static str,
    pub group: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub hidden: bool,
    /// Counter target; 0 = unlocks once.
    pub steps: u32,
    /// Play Games points (multiples of 5, 950 in all).
    pub pts: u32,
}

const fn a(group: &'static str, key: &'static str, name: &'static str, desc: &'static str, hidden: bool, steps: u32, pts: u32) -> Ach {
    Ach { key, group, name, desc, hidden, steps, pts }
}

const C: &str = "Classic";
const D: &str = "Solo and duo";
const R: &str = "City Run";
const M: &str = "City Run: more";
const K: &str = "Kills, streaks, score, power-ups";
const F: &str = "Just for fun";

pub const LIST: [Ach; 56] = [
    a(C, "first_blood", "First Blood", "Destroy your first enemy tank.", false, 0, 5),
    a(C, "stage_1", "Rookie", "Clear stage 1.", false, 0, 5),
    a(C, "stage_10", "Veteran", "Clear stage 10.", false, 0, 10),
    a(C, "stage_20", "Commander", "Clear stage 20.", false, 0, 20),
    a(C, "stage_30", "Last Stand", "Clear stage 30.", false, 0, 40),
    a(C, "second_lap", "Second Lap", "Clear stage 31: the maps start over, the enemies don't.", true, 0, 50),
    a(C, "flawless", "Flawless", "Clear a classic stage without losing a tank.", false, 0, 10),
    a(C, "iron_wall", "Iron Wall", "Clear 5 classic stages in a row without losing a tank, in one run.", false, 0, 30),
    a(D, "lone_wolf", "Lone Wolf", "Clear stage 20 in a one-player run.", false, 0, 30),
    a(D, "duo_stage", "Brothers in Arms", "Clear a classic stage with two players.", false, 0, 10),
    a(D, "duo_city", "City Duo", "Clear a City Run district with two players.", false, 0, 15),
    a(D, "lan_party", "LAN Party", "Clear a stage or district in LAN co-op.", false, 0, 15),
    a(D, "lifeline", "Lifeline", "Borrow a spare tank from your partner.", false, 0, 5),
    a(R, "city_1", "Off the Beaten Track", "Flattened the farms, the hedges and one very confused scarecrow.", false, 0, 5),
    a(R, "city_2", "Everything Must Go", "Clearance sale: the stalls, the arcades and the stallholders' tanks.", false, 0, 10),
    a(R, "city_3", "Lock, Stock and Barrel", "Took the locks, the bridges and the boss's last barrel.", false, 0, 10),
    a(R, "city_4", "Urban Renewal", "The church has a new skylight. You're welcome.", false, 0, 15),
    a(R, "city_5", "Strike While the Iron's Hot", "Cooled the ironworks down. Permanently.", false, 0, 15),
    a(R, "city_6", "Cool Under Fire", "Kept your head on the ice. Most of the other tanks didn't.", false, 0, 20),
    a(R, "city_7", "Dock Holiday", "Every container opened, every crane bent. Shipping is delayed.", false, 0, 20),
    a(R, "city_8", "Off the Rails", "The 5:15 to the Citadel is cancelled due to tanks on the line.", false, 0, 25),
    a(R, "city_9", "Gatecrasher", "Didn't knock.", false, 0, 30),
    a(R, "city_10", "Siege the Day", "The Citadel has fallen. Somebody tell the boss, he's still on fire.", false, 0, 40),
    a(M, "untouchable", "Untouchable", "Clear a district without losing a tank.", false, 0, 20),
    a(M, "ghost", "Ghost", "Clear The Citadel without losing a tank.", true, 0, 50),
    a(M, "digger", "Digger", "Open a secret stash.", false, 0, 5),
    a(M, "treasure_hunter", "Treasure Hunter", "Open all 3 secret stashes in one district.", false, 0, 15),
    a(M, "boss_slayer", "Boss Slayer", "Destroy 10 district bosses.", false, 10, 15),
    a(M, "wallbreaker", "Wallbreaker", "Destroy 100 bastion turrets.", false, 100, 15),
    a(K, "kills_100", "Tank Buster", "Destroy 100 enemy tanks.", false, 100, 10),
    a(K, "kills_1000", "Scrap Dealer", "Destroy 1,000 enemy tanks.", false, 1000, 25),
    a(K, "kills_5000", "Tank Graveyard", "Destroy 5,000 enemy tanks.", false, 5000, 50),
    a(K, "can_opener", "Can Opener", "Destroy 100 HEAVY tanks.", false, 100, 20),
    a(K, "streak_10", "Hot Streak", "Destroy 10 enemies in a row without losing a tank.", false, 0, 10),
    a(K, "streak_25", "Rampage", "Destroy 25 enemies in a row without losing a tank.", false, 0, 20),
    a(K, "streak_50", "Unstoppable", "Destroy 50 enemies in a row without losing a tank.", false, 0, 40),
    a(K, "score_20k", "Twenty Grand", "Score 20,000 points in one classic run.", false, 0, 10),
    a(K, "score_50k", "Fifty Grand", "Score 50,000 points in one classic run.", false, 0, 20),
    a(K, "score_100k", "Six Figures", "Score 100,000 points in one classic run.", false, 0, 40),
    a(K, "clean_sweep", "Clean Sweep", "Destroy 4 or more enemies with one grenade.", false, 0, 10),
    a(K, "fully_loaded", "Fully Loaded", "Reach star level 3.", false, 0, 5),
    a(
        F,
        "friends_like_these",
        "With Friends Like These",
        "Stun your partner with friendly fire and watch an enemy finish the job.",
        true,
        0,
        10,
    ),
    a(F, "star_thief", "Star Thief", "Take every star in a City Run district while your partner gets none.", true, 0, 15),
    a(F, "own_goal", "Own Goal", "Shoot your own base.", true, 0, 5),
    a(F, "borrowed_time", "Borrowed Time", "Take your partner's last spare tank and lose it within 10 seconds.", true, 0, 10),
    a(F, "loan_shark", "Loan Shark", "Borrow 5 tanks from your partner in one run.", false, 0, 10),
    a(
        F,
        "conscientious_objector",
        "Conscientious Objector",
        "Clear a classic stage in two-player without a single kill. Your partner did all 20.",
        true,
        0,
        10,
    ),
    a(F, "backseat_driver", "Backseat Driver", "Be out of tanks when your partner clears the stage alone.", true, 0, 5),
    a(F, "overkill", "Overkill", "Use a grenade on an empty field.", true, 0, 5),
    a(F, "six_pack", "Six Pack", "Destroy 6 enemies with one grenade.", true, 0, 15),
    a(F, "mutual_destruction", "Mutual Destruction", "You and an enemy destroy each other in the same moment.", true, 0, 10),
    a(F, "hanging_by_a_thread", "Hanging by a Thread", "Clear a classic stage with every brick of the base ring shot away.", false, 0, 10),
    a(F, "ff_enthusiast", "Friendly Fire Enthusiast", "Stun your partner 10 times in one stage.", false, 0, 5),
    a(F, "bathroom_break", "Bathroom Break", "Pause 10 times in one stage.", true, 0, 5),
    a(F, "bullet_ballet", "Bullet Ballet", "Cancel 100 enemy shells with your own.", false, 100, 10),
    // The hint to the title's secret combo (`ui::title::COMBO`), so it stays visible.
    a(F, "look_both_ways", "Look Both Ways", "On the title: look left, right, left again. Cross twice, then honk.", false, 0, 10),
];

/// The groups in list order with their short tab label.
pub const TABS: [(&str, &str); 6] = [(C, "CLASSIC"), (D, "DUO"), (R, "CITY"), (M, "CITY+"), (K, "COMBAT"), (F, "FUN")];

pub fn total_pts() -> u32 {
    LIST.iter().map(|a| a.pts).sum()
}

pub fn find(key: &str) -> Option<&'static Ach> {
    LIST.iter().find(|a| a.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_unique_and_fit_play_games() {
        for (i, x) in LIST.iter().enumerate() {
            assert!(LIST[..i].iter().all(|y| y.key != x.key), "{} twice", x.key);
            // Android resource names: lower case, digits, underscores.
            assert!(x.key.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'), "{}", x.key);
            assert!(x.name.len() <= 100 && x.desc.len() <= 500, "{}", x.key);
            assert!(x.pts > 0 && x.pts % 5 == 0, "{}", x.key);
            assert!(TABS.iter().any(|t| t.0 == x.group), "{}", x.key);
        }
        assert_eq!(total_pts(), 950, "Play Games allows 1000 per game");
    }
}
