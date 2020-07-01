export PGPASSWORD PGHOST
export PAGER=less

PGHOST=$(sudo -E grep rds /hab/svc/builder-api/config/config.toml | awk -F\" '{print $2}')
PGPASSWORD=$(sudo -E grep password /hab/svc/builder-api/config/config.toml | awk -F\" '{print $2}')

hab pkg exec core/postgresql psql -U hab -h "${PGHOST}" builder
