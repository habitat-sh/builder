# -*- mode: shell-script -*-
# shellcheck shell=bash

# shellcheck disable=SC2140
# {{#each cfg.env}}
export "{{@key}}"="{{this}}"
# {{/each }}

DEPRECATED_KEY_ID='{{cfg.key_id}}'
DEPRECATED_SECRET_KEY='{{cfg.secret_key}}'

if [ -n "$DEPRECATED_KEY_ID" ]; then
    echo 'Using deprecated key_id and secret_key options'
    echo 'Please replace it with new one:'
    echo '[env]'
    echo 'MINIO_ACCESS_KEY = "depot"'
    echo 'MINIO_SECRET_KEY = "password"'

    # shellcheck disable=SC2140
    export MINIO_ACCESS_KEY="$DEPRECATED_KEY_ID"
    export MINIO_SECRET_KEY="$DEPRECATED_SECRET_KEY"
fi

# AWS CLI is required to create bucket automatically
export AWS_ACCESS_KEY_ID="$MINIO_ACCESS_KEY"
export AWS_SECRET_ACCESS_KEY="$MINIO_SECRET_KEY"

MEMBERS='{{#if cfg.members }}{{strJoin cfg.members " "}}{{else}}{{pkg.svc_data_path}}{{/if}}'
BUCKET_NAME="{{cfg.bucket_name}}"

BIND_ADDRESS="{{cfg.bind_address}}"
BIND_PORT="{{cfg.bind_port}}"

create_bucket() {
    # When private.key file exists tls automatically is enabled
    if [ -f "{{pkg.svc_files_path}}/private.key" ]; then
        aws_s3="aws --endpoint-url https://localhost:$BIND_PORT --no-verify-ssl s3api"
    else
        aws_s3="aws --endpoint-url http://localhost:$BIND_PORT s3api"
    fi

    if $aws_s3 list-buckets | grep "$BUCKET_NAME" > /dev/null; then
        echo "Minio bucket is up to date."
    else
        echo "Creating minio bucket $BUCKET_NAME."
        $aws_s3 create-bucket --bucket "$BUCKET_NAME"
    fi
}
