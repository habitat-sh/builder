#!/bin/bash

set -euo pipefail
# insert into origin_package_settings (origin, name, visibility, owner_id) VALUES ($1, $2, $3, $4);
# 
# 
# id          | origin | name | visibility |      owner_id       |          created_at           |          updated_at           
# ---------------------+--------+------+------------+---------------------+-------------------------------+-------------------------------
 # 1610918355118596096 | core   | attr | public     | 1610917107782918144 | 2020-09-28 14:17:03.638064+00 | 2020-09-28 14:17:03.638064+00
# (1 row)
# 
         # id          |   origin    | package_name |          name          |     plan_path      |      owner_id       | vcs_type |                          vcs_data                           |          created_at          |          updated_at          | vcs_installation_id | visibility | auto_build |    target    
# ---------------------+-------------+--------------+------------------------+--------------------+---------------------+----------+-------------------------------------------------------------+------------------------------+------------------------------+---------------------+------------+------------+--------------
# 1610918628402667520 | smacfarlane | empty-plan   | smacfarlane/empty-plan | empty-plan/plan.sh | 1610917107782918144 | git      | https://github.com/smacfarlane/habitat-empty-test-plans.git | 2020-09-28 14:17:36.21613+00 | 2020-09-28 14:17:36.21613+00 |             3189516 | public     | t          | x86_64-linux
# (1 row)

account="smacfarlane"
vcs_id='3189516' # Unique to the github app(?)

for plan in $(< cycle-list); do 

	name="$plan"
	plan_path="$plan/plan.sh"

	cat << EOS
-- START $name
with new_owner_id as (
  select id from accounts where name='$account'
),
new_package as (
insert into origin_package_settings (origin, name, visibility, owner_id) values 
  (
    'core', 
    '$name', 
    'public', 
    (select id from new_owner_id)
  )
  returning id
)
insert into origin_projects (origin, package_name, name, plan_path, owner_id, vcs_type, vcs_data, vcs_installation_id, visibility, auto_build, target) VALUES 
  (
    'core',
    '$name',
    'core/$name',
    '$plan_path',
    (select id from new_package),
    'git',
    'https://github.com/smacfarlane/builder-cyclic-test-plans',
    '$vcs_id',
    'public',
    't',
    'x86_64-linux'
);
-- END $name
EOS
done
