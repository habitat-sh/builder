#!/bin/bash

set -euo pipefail

aggMsg() {
    echo ""
    echo "Aggregating remotely stored artifacts."
    echo "======================================"
    echo ""

}

configMsg(){
    echo ""
    echo "Configuring environment for dep ingestion."
    echo "======================================"
    echo ""
}

keyMsg() {
    echo ""
    echo "Generating S3 Object Keys."
    echo "======================================"
    echo ""
}

uploadMsg() {
    echo ""
    echo "Uploading hartfiles."
    echo "======================================"
}

getLatest() {
    curl -s "${bldr_url}/v1/depot/channels/core/stable/pkgs/${1}/latest" | jq -r '.ident | [.origin,.name,.version,.release] | join("/")'
}

getHarts() {
    # Traverses Core path in S3 bucket and
    # creates an array from each package
    # artifact that exists in that path
    origin="core"
    # shellcheck disable=SC2207
    core_dirs=( $(aws --endpoint-url "${region}" s3 ls "s3://${bucket}/${origin}/") )
   
    for dir in "${core_dirs[@]}"; do
        if [[ "${dir}" != "PRE" ]]; then
            echo package entry found for "${dir}"
            artifacts+=( "$(getLatest "${dir}")" )
        fi;
    done

    echo ""
    echo "Downloading latest artifacts"
    echo "This may take a moment"
    echo ""

    for hart in "${artifacts[@]}"; do
        echo "${hart}"
        downloadHarts "${region}" "${bucket}" "${hart}"
    done
}

downloadHarts() {
    download_path="/tmp/harts/"
    mkdir -p "${download_path}"
    aws --endpoint-url "${1}" s3 cp --recursive "s3://${2}/${3}" "${download_path}"
}

checkBucket() {
    region="${1}"
    bucket="${2}"
    aws --endpoint-url "${region}" s3api list-objects --bucket "${bucket}"
}

installDeps() {
    PDEPS=("core/aws-cli" "core/jq-static" "habitat/s3-bulk-uploader")

    configMsg

    for bin in "${!PDEPS[@]}"; do
        HAB_AUTH_TOKEN="" hab pkg install -b "${PDEPS[$bin]}"
    done
}

setBucket() {
    # Configure the bucket name to use for the upload
    sgroups=("default" "dev" "prod" "acceptance" "live" "blue" "green")
    for i in "${sgroups[@]}"; do
        if curl -s localhost:9631/services/builder-api/"${i}" > /dev/null; then
            bucket_name=$(curl -s localhost:9631/services/builder-api/"${i}" | jq .cfg.s3.bucket_name)
            if [[ -n $bucket_name ]]; then
                echo ""
                echo "We've detected your minio bucket configuration set to: ${bucket_name}!"
                read -r -p "Would you like to use this minio bucket? [y/N] " response
                if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
                    echo "Setting bucket to ${bucket_name}"
                    bucket_name="${bucket_name//\"}"
                    break
                fi
            fi
        fi
    done

    if [[ -z ${bucket_name:-} ]]; then
      echo ""
      echo "Please enter a target bucket name and press [ENTER]:"
      read -r bucket_name
    fi

    # Check if bucket exists
    if [ "${s3type}" == 'minio' ]; then
        if checkBucket "${AWS_REGION}" "${bucket_name}" >/dev/null; then
            echo ""
            echo "Bucket: ${bucket_name} found!"
            echo ""
            read -r -p "Are you sure you would like to use this bucket? [y/N] " response
            if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
                echo "Using specified bucket."
                export S3_BUCKET=${bucket_name}
            else
                setBucket
            fi
        else
            echo "Bucket: ${bucket_name} not found!"
            echo "Please specify a different bucket and try again."
            setBucket
        fi
    else
        if aws s3api list-objects --bucket "${bucket_name}" --region "${AWS_REGION}" >/dev/null; then
            echo "Bucket: ${bucket_name} found!"
            echo "WARNING: Specified bucket is not empty!"
            read -r -p "Are you sure you would like to use this bucket? [y/N] " response
            if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
                echo "Using specified bucket."
                export S3_BUCKET=${bucket_name}
            else
                setBucket
            fi
        else
            echo "Bucket: ${bucket_name} not found!"
            echo "Please specify a different bucket and try again."
            setBucket
        fi
    fi
}

checkBucket() {
    region="${1}"
    bucket="${2}"
    aws --endpoint-url "${region}" s3api list-objects --bucket "${bucket}"
}

setRegion() {
    # Sets the region used by the bucket
    if [ "${s3type}" == 'minio' ]; then
        sgroups=("default" "dev" "prod" "acceptance" "live" "blue" "green")
        for i in "${sgroups[@]}"; do
            if curl -s localhost:9631/services/builder-minio/"${i}" > /dev/null; then
                minioIP=$(curl -s localhost:9631/services/builder-minio/"${i}" | jq .sys.ip)
                echo ""
                echo "We've detected your minio instance at: ${minioIP}!"
                echo ""
                read -r -p "Would you like to use this minio instance? [y/N] " response
                if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
                    echo "Setting endpoint to ${minioIP//\"}:9000"
                    if curl -s localhost:9631/services/builder-minio/"${i}" | jq .cfg.use_ssl | grep "true"> /dev/null; then
                        export AWS_REGION="https://${minioIP//\"}:9000"
                    else
                        export AWS_REGION="http://${minioIP//\"}:9000"
                    fi
                    return
                else
                    echo ""
                    echo "==========================================================="
                    echo "Please enter the minio endpoint URI and press [ENTER]:"
                    echo "(http://localhost:9000 || https://10.1.250.4:9000)"
                    echo "==========================================================="
                    read -r region_name
                    export AWS_REGION=${region_name}
                    return
                fi
            fi
        done
    else
        echo ""
        echo "==========================================================="
        echo "Please enter the region for your bucket and press [ENTER]:"
        echo "(us-west-1 us-east-1 us-west-2 etc )"
        echo "==========================================================="
        read -r region_name
        export AWS_REGION=${region_name}
    fi
}

uploadHarts() {
    # Takes the artifact array generated via
    # traversing the data path and uploads to s3
    uploadMsg
    putArtifacts
    echo ""
    echo "########################################"
    echo "${#artifacts[@]} Artifacts have been ingested!"
    echo "########################################"
    echo ""
}

putArtifacts() {
    for file in $(find /tmp/harts/ -name '*.hart'); do
        hab pkg upload --url "${bldr_url}" "${file}" --force
    done
}

genS3Config() {
    echo ""
    read -r -p "AWS Access Key ID: " access_key_id
        export AWS_ACCESS_KEY_ID="${access_key_id}"
    read -r -p "AWS Secret Access Key: " secret_access_key
        export AWS_SECRET_ACCESS_KEY="${secret_access_key}"
}

genBldrCreds() {
    echo ""
    echo "Please provide a Builder Auth Token."
    read -r -p "Builder Auth token: " bldr_auth_token
        export HAB_AUTH_TOKEN="${bldr_auth_token}"
    echo ""
    echo "Builder auth token set to: ${HAB_AUTH_TOKEN}"
    read -r -p "Would you like to use this auth token? [y/N]" response
    if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
        return
    else
        genBldrCreds
    fi
}

setBldrUrl() {
    echo ""
    echo "Package ingestion can be pointed towards public builder or"
    echo "an on-prem builder instance."

    if [[ -n ${HAB_BLDR_URL:-} ]]; then
       echo ""
       echo "Builder URL configured via ENVVAR detected."
       echo ""
       echo "HAB_BLDR_URL=${HAB_BLDR_URL:-}"
       export bldr_url="${HAB_BLDR_URL}"
    else
        echo ""
        read -r -p "Will you be uploading to public builder? [y/N]" response
            if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
                export bldr_url="https://bldr.habitat.sh"
            else
                echo ""
                echo "Please provide the URL so your builder instance"
                echo "Ex: https://bldr.habitat.sh or http://localhost"
                read -r -p ": " response
                    export bldr_url="${response}"
                echo ""
                echo "Your builder URL is now configured to ${bldr_url}"
                read -r -p "Is this correct? [y/N]" response
                    if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
                        return
                    else
                        setBldrUrl
                    fi
        fi
    fi
}

credSelect() {
    echo ""
    read -r -p "Would you like to use these credentials? [y/N] " response
    if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
        return
    else
        echo ""
        read -r -p "Would you like to configure with custom credentials now? [y/N]" response
        if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
            genS3Config
        else
            echo "Please reconfigure your AWS credentials and re-run s3migrate."
            exit 1
        fi
    fi
}

credCheck(){
    CREDS=( "${HOME}/.aws/credentials" "${HOME}/.aws/config" "/root/.aws/credentials" "/root/.aws/config")
    for location in "${CREDS[@]}"; do
        if [ -f "${location}" ]; then
            echo ""
            echo "AWS Credentials file located at ${location}"
            cat "${location}"
            credSelect
            credsConfigured=true
            break
        fi

        if [[ -n ${AWS_ACCESS_KEY_ID:-} ]]; then
            if [[ -n ${AWS_SECRET_ACCESS_KEY:-} ]]; then
                echo ""
                echo "AWS Credentials configured via ENVVAR detected."
                echo ""
                echo "aws_access_key_id=${AWS_ACCESS_KEY_ID:-}"
                echo "aws_secret_access_key=${AWS_SECRET_ACCESS_KEY:-}"
                credSelect
                credsConfigured=true
                break
            else
                echo ""
                echo "WARNING: Incomplete AWS Credentials configured via ENVVAR."
                echo "Make sure to set AWS_ACCESS_KEY_ID && AWS_SECRET_ACCESS_KEY"
                break
            fi
        fi
    done

    if  "${credsConfigured}" ; then
        echo ""
        echo "Credentials configured!"
    else
        echo ""
        echo "WARNING: No AWS credentials detected!"
        read -r -p "Would you like to generate them now? [y/N] " response
        if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
            genS3Config
        else
            echo "Please configure your AWS credentials and re-run s3migrate."
            echo ""
            exit 1
        fi
    fi
}

welcome() {
    echo ""
    echo "==========================================================="
    echo "###########################################################"
    echo "############## Bldr Build Dep Ingest/Migrate ##############"
    echo "###########################################################"
    echo "==========================================================="
    echo ""
    echo "This tool will scrape all the hart files from an s3 or minio"
    echo "bucket and reingest them to populate build dep and tdep metadata"
    credsConfigured=false

    setBldrUrl



    if [[ -n "${HAB_AUTH_TOKEN+x}" ]]; then
        echo ""
        echo "We were able to discover your builder auth token as: "
        echo "${HAB_AUTH_TOKEN}"
        read -r -p "Would you like to use these credentials? [y/N]" response
            if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
                echo "Setting detected credentials"
            fi
    else
        echo ""
        echo "WARNING: No Builder credentials detected!"
        read -r -p "Would you like to generate them now? [y/N] " response
        if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
            genBldrCreds
        else
            echo "Please configure your builder auth-token and re-run s3migrate."
            echo ""
            exit 1
        fi
    fi

    if [ "$s3type" = "minio" ]; then
        echo ""
        echo "It looks like you specified an ingestion from minio!"

        sgroups=("default" "dev" "prod")
        for i in "${sgroups[@]}"; do
            if curl -s localhost:9631/services/builder-minio/"${i}" | jq .cfg.key_id > /dev/null; then
                access_key_id=$(curl -s localhost:9631/services/builder-minio/default | jq .cfg.key_id)
                secret_access_key=$(curl -s localhost:9631/services/builder-minio/default | jq .cfg.secret_key)
                echo ""
                echo "We were able to detect your minio credentials!"
                echo "(ACCESS_KEY_ID) Username: ${access_key_id}"
                echo "(SECRET ACCESS_KEY) Password: ${secret_access_key}"
                echo ""
                read -r -p "Would you like to use these credentials? [y/N] " response
                    if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
                        echo "Setting detected credentials"
                        export AWS_ACCESS_KEY_ID=${access_key_id//\"}
                        export AWS_SECRET_ACCESS_KEY=${secret_access_key//\"}
                        return
                    else
                        credCheck
                    fi
            else
                echo ""
                echo "Minio will use whatever credentials you've configured it with."
                echo "If those credentials don't match your aws credentials file, you"
                echo "must specify those custom credentials."
                credCheck
            fi
        done
    else
        credCheck
    fi

}

artifacts=()

if [[ -z ${1:-} ]]; then
    echo "Invalid Argument. Argument must be either 'minio' or 'aws'"
    exit 1
fi

case ${1} in
    'minio')
        echo "Starting ingestion from minio instance."
        export s3type="minio"
        installDeps
        welcome
        setRegion
        setBucket
        getHarts 
        time uploadHarts
    ;;
    'aws')
        echo "Starting ingestion  AWS S3."
        export s3type="aws"
        installDeps
        welcome
        setRegion
        setBucket
        getHarts 
        time uploadHarts
    ;;
    *) echo "Invalid argument. Arg must be 'minio' or 'aws'"
        exit 1
    ;;
esac

