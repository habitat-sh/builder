pkg_origin="habitat"
pkg_name="builder"
pkg_type="composite"
pkg_version="0.1.0"

pkg_services=(
    habitat/builder-api
    habitat/builder-api-proxy
    habitat/builder-datastore
    habitat/builder-jobsrv
    habitat/builder-worker
)

pkg_bind_map=(
    [habitat/builder-api-proxy]="http:habitat/builder-api"
    [habitat/builder-jobsrv]="datastore:habitat/builder-datastore"
    [habitat/builder-worker]="jobsrv:habitat/builder-jobsrv depot:habitat/builder-api"
)
