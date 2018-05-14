#!/bin/sh
# Provides id and key instead of generating one
export MINIO_ACCESS_KEY="{{cfg.key_id}}"
export MINIO_SECRET_KEY="{{cfg.secret_key}}"
