#!/bin/bash

set -euo pipefail

echoMsg() {
    echo ""
    echo "${1}"
    echo "======================================"
    echo ""
}

configMsg(){
    echoMsg "Configuring environment for dep ingestion."
}

uploadMsg() {
    echoMsg "Uploading hartfiles."
}

strip_double_quotes() {
    printf "%s" "${1//\"}"
}

promptUser() {
    local response
    read -r -p "${1} [y/N] " response
    if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
        true
    else
        false
    fi
}

getLatest() {
   curl -s "${bldr_url}"/v1/depot/channels/core/stable/pkgs/"${1}"/latest | jq -r '.ident | [.origin,.name,.version,.release] | join("/")'
}

setRegion() {
    # Sets the region used by the bucket
    if [[ "${s3type}" == 'minio' ]]; then
        sgroups=("default" "dev" "prod" "acceptance" "live" "blue" "green")
        for i in "${sgroups[@]}"; do
            if curl -s localhost:9631/services/builder-minio/"${i}" > /dev/null; then
                minioIP=$(curl -s localhost:9631/services/builder-minio/"${i}" | jq .sys.ip)
                echo ""
                echo "We've detected your minio instance at: ${minioIP}!"
                echo ""
                if promptUser "Would you like to use this minio instance?"; then
                    # This pattern `//\"` strips the double quotes from an interpolated string.
                    # we use this regularly throughout this script
                    echo "Setting endpoint to ${minioIP//\"}:9000"
                    if curl -s localhost:9631/services/builder-minio/"${i}" | jq .cfg.use_ssl | grep "true"> /dev/null; then
                        AWS_REGION="https://$(strip_double_quotes "$minioIP"):9000"
                        export AWS_REGION
                    else
                        AWS_REGION="http://$(strip_double_quotes "$minioIP"):9000"
                        export AWS_REGION
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

getHarts() {
    # Traverses Core path in S3 bucket and
    # creates an array from each package
    # artifact that exists in that path
    origin="core"
    mapfile -t core_dirs < <(aws "${region_option}" "${AWS_REGION}" s3 ls "s3://${S3_BUCKET}/${origin}/" | awk '{print $2}')

    for dir in "${core_dirs[@]}"; do
            # shellcheck disable=SC2207
            artifacts+=( $(curl -s "${bldr_url}"/v1/depot/channels/core/stable/pkgs/"${dir}"latest | jq -r '.ident | [.origin,.name,.version,.release] | join("/")') )
    done
    echo ""
    echo "Downloading latest artifacts"
    echo "This may take a moment"
    echo ""
    na

    for hart in "${artifacts[@]}"; do
        downloadHarts "${hart}"
    done
}

downloadHarts() {
    download_path="/hab/tmp/harts/"
    mkdir -p "${download_path}"

    aws "${region_option}" "${AWS_REGION}" s3 cp --recursive "s3://${S3_BUCKET}/${1}" "${download_path}"
}

checkBucket() {
    aws "${region_option}" "${AWS_REGION}" s3api list-objects --bucket "${bucket_name}" >/dev/null
}

installDeps() (
    pdeps=("core/aws-cli" "core/jq-static" "habitat/s3-bulk-uploader")
    unset HAB_AUTH_TOKEN

    configMsg

    for bin in "${pdeps[@]}"; do
        hab pkg install -b "${bin}"
    done
)

setBucket() {
    # Configure the bucket name to use for the upload
    sgroups=("default" "dev" "prod" "acceptance" "live" "blue" "green")
    for i in "${sgroups[@]}"; do
        if services=$(curl -s localhost:9631/services/builder-api/"${i}"); then
            bucket_name=$(echo "$services" | jq .cfg.s3.bucket_name)
            if [[ -n $bucket_name ]]; then
                echo ""
                echo "We've detected your minio bucket configuration set to: ${bucket_name}!"
                if promptUser "Would you like to use this minio bucket?"; then
                    echo "Setting bucket to ${bucket_name}"
                    bucket_name="$(strip_double_quotes "$bucket_name")"
                    break
                fi
            fi
        fi
    done

    if [[ -z "${bucket_name}" ]]; then
      echo ""
      echo "Please enter a target bucket name and press [ENTER]:"
      read -r bucket_name
    fi

    # Check if bucket exists
    if [[ "${s3type}" == 'minio' ]]; then
        if checkBucket; then
            echo ""
            echo "Configured bucket: ${bucket_name} has been verified!"
            echo ""
            if promptUser "Are you sure you would like to use this bucket?"; then 
                echo "Using specified bucket."
                S3_BUCKET=${bucket_name}
            else
                setBucket
            fi
        else
            echo "Configured Bucket: ${bucket_name} was not found!"
            echo "Please specify a different bucket and try again."
            setBucket
        fi
    else
        if aws s3api list-objects --bucket "${bucket_name}" --region "${AWS_REGION}" >/dev/null; then
            echo "Bucket: ${bucket_name} found!"
            echo "WARNING: Specified bucket is not empty!"
            if promptUser "Are you sure you would like to use this bucket?"; then
                echo "Using specified bucket."
                S3_BUCKET=${bucket_name}
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



failed_uploads=()
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

    if [[ "${#failed_uploads[@]}" -gt 0 ]]; then
        echo ""
        echo "########################################"
        echo "The following artifacts failed on upload:"
        for failure in "${failed_uploads[@]}"; do
            echo "${failure}"
        done
        echo "########################################"
    fi
}

putArtifacts() {
    while IFS= read -r -d '' file; do
        if ! hab pkg upload --url "${bldr_url}" "${file}" --force; then
            failed_uploads+=("${file}")
        fi
    done < <(find /hab/tmp/harts/ -name '*.hart' -print0)
}

genS3Config() {
    echo ""
    read -r -p "AWS Access Key ID: " AWS_ACCESS_KEY_ID
        export AWS_ACCESS_KEY_ID
    read -r -p "AWS Secret Access Key: " AWS_SECRET_ACCESS_KEY
        export AWS_SECRET_ACCESS_KEY
}

genBldrCreds() {
    echo ""
    echo "Please provide a Builder Auth Token."
    read -r -p "Builder Auth token: " HAB_AUTH_TOKEN
        export HAB_AUTH_TOKEN
    echo ""
    echo "Builder auth token set to: ${HAB_AUTH_TOKEN}"
    if promptUser "Would you like to use this auth token?"; then
        return
    else
        genBldrCreds
    fi
}

setBldrUrl() {
    echo ""
    echo "Package ingestion can be pointed towards public builder or"
    echo "an on-prem builder instance."

    if [[ -n "${HAB_BLDR_URL:+x}" ]]; then
       echo ""
       echo "Builder URL configured via ENVVAR detected."
       echo ""
       echo "HAB_BLDR_URL=${HAB_BLDR_URL}"
       bldr_url="${HAB_BLDR_URL}"
    else
        echo ""
        if promptUser "Will you be uploading to public builder?"; then
                export bldr_url="https://bldr.habitat.sh"
        else
            echo ""
            echo "Please provide the URL so your builder instance"
            echo "Ex: https://bldr.habitat.sh or http://localhost"
            read -r -p ": " response
                bldr_url="${response}"
            echo ""
            echo "Your builder URL is now configured to ${bldr_url}"
            if promptUser "Is this correct?"; then
                return
            else
                setBldrUrl
            fi
        fi
    fi
}

credSelect() {
    echo ""
    if promptUser "Would you like to use these credentials?"; then
        return
    else
        echo ""
        if promptUser "Would you like to configure with custom credentials now?"; then
            genS3Config
        else
            echo "Please reconfigure your AWS credentials and re-run s3migrate."
            exit 1
        fi
    fi
}

credCheck(){
    creds=( "${HOME}/.aws/credentials" "${HOME}/.aws/config" "/root/.aws/credentials" "/root/.aws/config")
    for location in "${creds[@]}"; do
        if [[ -f "${location}" ]]; then
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
                echo "aws_access_key_id=${AWS_ACCESS_KEY_ID}"
                echo "aws_secret_access_key=${AWS_SECRET_ACCESS_KEY}"
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
        if promptUse "Would you like to generate them now?"; then
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

    if [[ -n "${HAB_AUTH_TOKEN:-}" ]]; then
        echo ""
        echo "We were able to discover your builder auth token as: "
        echo "${HAB_AUTH_TOKEN}"
        if promptUser "Would you like to use these credentials?"; then
           echo "Setting detected credentials"
       else 
           genBldrCreds
        fi
    else
        echo ""
        echo "WARNING: No Builder credentials detected!"
        if promptUser "Would you like to generate them now?"; then
            genBldrCreds
        else
            echo "Please configure your builder auth-token and re-run $(basename "${0}")."
            echo ""
            exit 1
        fi
    fi

    if [[ "$s3type" = "minio" ]]; then
        echo ""
        echo "It looks like you specified an ingestion from minio!"

        sgroups=("default" "dev" "prod")
        for i in "${sgroups[@]}"; do
            if minio_output=$(curl -s localhost:9631/services/builder-minio/"${i}" ); then
                access_key_id=$(echo "$minio_output" | jq .cfg.key_id)
                secret_access_key=$(jq .cfg.secret_key <<< "$minio_output")
                echo ""
                echo "We were able to detect your minio credentials!"
                echo "(ACCESS_KEY_ID) Username: ${access_key_id}"
                echo "(SECRET ACCESS_KEY) Password: ${secret_access_key}"
                echo ""
                if promptUser "Would you like to use these credentials?"; then
                    echo "Setting detected credentials"
                    AWS_ACCESS_KEY_ID="$(strip_double_quotes "$access_key_id")"
                    export AWS_ACCESS_KEY_ID
                    AWS_SECRET_ACCESS_KEY="$(strip_double_quotes "$secret_access_key")"
                    export AWS_SECRET_ACCESS_KEY
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
bucket_name=""
S3_BUCKET=""

case ${1:-} in
    'minio')
        echo "Starting ingestion from minio instance."
        s3type="minio"
        region_option="--endpoint-url"
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
        export region_option="--region"
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
