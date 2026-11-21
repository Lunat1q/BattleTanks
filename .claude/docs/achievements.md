# Achievements (planned, ADR 0017)

55 achievements, 940 points (Play Games caps a game at 1000; points are multiples of 5). `key` is stable: it names
the host's id (on Android `R.string.achievement_<key>`) and the line in `achievements.txt`, so never rename one.
Counters show progress (`steps`); the rest unlock once. Hidden ones show as `???` until unlocked.

Common rules:
- Only the players on this device count: on a couch both joined slots (one signed-in account), over LAN only this
  machine's slot (`net.me()`). Assisted runs (KEEP POWER, FRIENDLY FIRE off) count like any other.
- "Kill" = an ordinary enemy tank destroyed by your shell (grenade kills only count for the grenade achievements).
  Turrets, mini-bosses and bosses have their own counters.
- "Without losing a tank" = none of your tanks destroyed in that stage / district (a stun is not a loss).
- A streak counts your kills since your last lost tank, across stages, and ends with the run.
- Score = your own score in one classic run (not the team total).

## Classic
| key | Name | Description | Hidden | Steps | Pts |
|---|---|---|---|---|---|
| `first_blood` | First Blood | Destroy your first enemy tank. | | | 5 |
| `stage_1` | Rookie | Clear stage 1. | | | 5 |
| `stage_10` | Veteran | Clear stage 10. | | | 10 |
| `stage_20` | Commander | Clear stage 20. | | | 20 |
| `stage_30` | Last Stand | Clear stage 30. | | | 40 |
| `second_lap` | Second Lap | Clear stage 31: the maps start over, the enemies don't. | yes | | 50 |
| `flawless` | Flawless | Clear a classic stage without losing a tank. | | | 10 |
| `iron_wall` | Iron Wall | Clear 5 classic stages in a row without losing a tank, in one run. | | | 30 |

## Solo and duo
| key | Name | Description | Hidden | Steps | Pts |
|---|---|---|---|---|---|
| `lone_wolf` | Lone Wolf | Clear stage 20 in a one-player run. | | | 30 |
| `duo_stage` | Brothers in Arms | Clear a classic stage with two players. | | | 10 |
| `duo_city` | City Duo | Clear a City Run district with two players. | | | 15 |
| `lan_party` | LAN Party | Clear a stage or district in LAN co-op. | | | 15 |
| `lifeline` | Lifeline | Borrow a spare tank from your partner. | | | 5 |

## City Run: one per district (boss killed in district N)
| key | District | Name | Description | Pts |
|---|---|---|---|---|
| `city_1` | Outskirts | Off the Beaten Track | Flattened the farms, the hedges and one very confused scarecrow. | 5 |
| `city_2` | Market Row | Everything Must Go | Clearance sale: the stalls, the arcades and the stallholders' tanks. | 10 |
| `city_3` | Canal Ward | Lock, Stock and Barrel | Took the locks, the bridges and the boss's last barrel. | 10 |
| `city_4` | Old Town | Urban Renewal | The church has a new skylight. You're welcome. | 15 |
| `city_5` | Ironworks | Strike While the Iron's Hot | Cooled the ironworks down. Permanently. | 15 |
| `city_6` | Frost Park | Cool Under Fire | Kept your head on the ice. Most of the other tanks didn't. | 20 |
| `city_7` | Harbour | Dock Holiday | Every container opened, every crane bent. Shipping is delayed. | 20 |
| `city_8` | Rail Yards | Off the Rails | The 5:15 to the Citadel is cancelled due to tanks on the line. | 25 |
| `city_9` | Citadel Gate | Gatecrasher | Didn't knock. | 30 |
| `city_10` | The Citadel | Siege the Day | The Citadel has fallen. Somebody tell the boss, he's still on fire. | 40 |

## City Run: more
| key | Name | Description | Hidden | Steps | Pts |
|---|---|---|---|---|---|
| `untouchable` | Untouchable | Clear a district without losing a tank. | | | 20 |
| `ghost` | Ghost | Clear The Citadel without losing a tank. | yes | | 50 |
| `digger` | Digger | Open a secret stash. | | | 5 |
| `treasure_hunter` | Treasure Hunter | Open all 3 secret stashes in one district. | | | 15 |
| `boss_slayer` | Boss Slayer | Destroy 10 district bosses. | | 10 | 15 |
| `wallbreaker` | Wallbreaker | Destroy 100 bastion turrets. | | 100 | 15 |

## Kills, streaks, score, power-ups
| key | Name | Description | Hidden | Steps | Pts |
|---|---|---|---|---|---|
| `kills_100` | Tank Buster | Destroy 100 enemy tanks. | | 100 | 10 |
| `kills_1000` | Scrap Dealer | Destroy 1,000 enemy tanks. | | 1000 | 25 |
| `kills_5000` | Tank Graveyard | Destroy 5,000 enemy tanks. | | 5000 | 50 |
| `can_opener` | Can Opener | Destroy 100 HEAVY tanks. | | 100 | 20 |
| `streak_10` | Hot Streak | Destroy 10 enemies in a row without losing a tank. | | | 10 |
| `streak_25` | Rampage | Destroy 25 enemies in a row without losing a tank. | | | 20 |
| `streak_50` | Unstoppable | Destroy 50 enemies in a row without losing a tank. | | | 40 |
| `score_20k` | Twenty Grand | Score 20,000 points in one classic run. | | | 10 |
| `score_50k` | Fifty Grand | Score 50,000 points in one classic run. | | | 20 |
| `score_100k` | Six Figures | Score 100,000 points in one classic run. | | | 40 |
| `clean_sweep` | Clean Sweep | Destroy 4 or more enemies with one grenade. | | | 10 |
| `fully_loaded` | Fully Loaded | Reach star level 3. | | | 5 |

## Just for fun (credited to the player who did it)
| key | Name | Description | Hidden | Steps | Pts |
|---|---|---|---|---|---|
| `friends_like_these` | With Friends Like These | Stun your partner with friendly fire and watch an enemy finish the job. | yes | | 10 |
| `star_thief` | Star Thief | Take every star in a City Run district while your partner gets none. | yes | | 15 |
| `own_goal` | Own Goal | Shoot your own base. | yes | | 5 |
| `borrowed_time` | Borrowed Time | Take your partner's last spare tank and lose it within 10 seconds. | yes | | 10 |
| `loan_shark` | Loan Shark | Borrow 5 tanks from your partner in one run. | | | 10 |
| `conscientious_objector` | Conscientious Objector | Clear a classic stage in two-player without a single kill. Your partner did all 20. | yes | | 10 |
| `backseat_driver` | Backseat Driver | Be out of tanks when your partner clears the stage alone. | yes | | 5 |
| `overkill` | Overkill | Use a grenade on an empty field. | yes | | 5 |
| `six_pack` | Six Pack | Destroy 6 enemies with one grenade. | yes | | 15 |
| `mutual_destruction` | Mutual Destruction | You and an enemy destroy each other in the same moment. | yes | | 10 |
| `hanging_by_a_thread` | Hanging by a Thread | Clear a classic stage with every brick of the base ring shot away. | | | 10 |
| `ff_enthusiast` | Friendly Fire Enthusiast | Stun your partner 10 times in one stage. | | | 5 |
| `bathroom_break` | Bathroom Break | Pause 10 times in one stage. | yes | | 5 |
| `bullet_ballet` | Bullet Ballet | Cancel 100 enemy shells with your own. | | 100 | 10 |
