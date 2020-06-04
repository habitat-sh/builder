#!/bin/bash

# Post a Datadog event indicating a merge to master occurred

set -eou pipefail

DD_CLIENT_API_KEY=$(vault kv get -field api_key_acceptance account/static/datadog/habitat)
export DD_CLIENT_API_KEY

curl --connect-timeout 5 \
  --max-time 10 \
  --retry 5 \
  --retry-delay 0 \
  --retry-max-time 40 \
  --request POST https://api.datadoghq.com/api/v1/events \
  --header "Expect:" \
  --header "DD-API-KEY: ${DD_CLIENT_API_KEY}" \
  --header 'Content-Type: application/json charset=utf-8' \
  --data-binary @- << EOF
{  "aggregation_key":"git_merge",
   "alert_type":"info",
   "date_happened":$(date "+%s"),
   "priority":"normal",
   "source_type_name":"GIT",
   "tags":[
      "environment:acceptance"
],
   "text":"'${EXPEDITOR_TITLE}' https://github.com/habitat-sh/builder/pull/${EXPEDITOR_NUMBER} was merged with merge commit ${EXPEDITOR_MERGE_COMMIT}",
   "title":"Builder PR ${EXPEDITOR_NUMBER} merged to master"
}
EOF
