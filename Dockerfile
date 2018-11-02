FROM alpine:3.6
MAINTAINER The Habitat Maintainers <humans@habitat.sh>

ARG APP_HOSTNAME=localhost
ARG GITHUB_ADDR=github.com
ARG GITHUB_API_URL=https://api.github.com
ARG GITHUB_AUTHORIZE_URL=https://github.com/login/oauth/authorize
ARG GITHUB_TOKEN_URL=https://github.com/login/oauth/access_token
ARG GITHUB_CLIENT_ID=UNDEFINED
ARG GITHUB_CLIENT_SECRET=UNDEFINED
ARG GITHUB_APP_ID=UNDEFINED
ARG GITHUB_APP_URL=https://github.com/apps/habitat-builder-dev-studio

ENV HAB_BLDR_CHANNEL unstable
ENV RUST_LOG info

COPY support/builder/config.sh /tmp/config.sh
COPY support/builder/datastore.toml /hab/svc/builder-datastore/user.toml
COPY support/builder/hab-entrypoint.sh /usr/local/bin/hab-entrypoint.sh
COPY support/builder/init-datastore.sh /tmp/init-datastore.sh
COPY terraform/scripts/install_base_packages.sh /tmp/install_base_packages.sh
COPY terraform/scripts/foundation.sh /tmp/foundation.sh
COPY .secrets/builder-dev-app.pem /hab/svc/builder-api/files/builder-github-app.pem

RUN adduser -g tty -h /home/krangschnak -D krangschnak \
  && addgroup -S hab && adduser -S -G hab hab \
  && apk add --no-cache \
  bash \
  curl \
  perl-utils \
  && /tmp/install_base_packages.sh \
  && rm -Rf hab_builder_bootstrap* hab_bootstrap* LATEST 0 \
  && hab pkg install core/hab -c unstable -b \
  && hab pkg install core/hab-sup \
  core/hab-launcher \
  habitat/builder-datastore \
  habitat/builder-api \
  habitat/builder-api-proxy \
  habitat/builder-jobsrv\

RUN /tmp/init-datastore.sh \
  && APP_HOSTNAME=$APP_HOSTNAME \
  GITHUB_ADDR=$GITHUB_ADDR \
  GITHUB_API_URL=$GITHUB_API_URL \
  GITHUB_AUTHORIZE_URL=$GITHUB_AUTHORIZE_URL \
  GITHUB_TOKEN_URL=$GITHUB_TOKEN_URL \
  GITHUB_CLIENT_ID=$GITHUB_CLIENT_ID \
  GITHUB_CLIENT_SECRET=$GITHUB_CLIENT_SECRET \
  GITHUB_APP_ID=$GITHUB_APP_ID \
  GITHUB_APP_URL=$GITHUB_APP_URL \
  /tmp/config.sh

RUN hab pkg exec core/openssl openssl s_client -showcerts -connect $GITHUB_ADDR:443 \
  </dev/null 2>/dev/null|hab pkg exec core/openssl openssl x509 -outform PEM >> \
  /usr/local/share/ca-certificates/github.crt && update-ca-certificates

RUN hab svc load habitat/builder-datastore \
  && hab svc load habitat/builder-api-proxy --bind http:builder-api.default \
  && hab svc load habitat/builder-api --bind datastore:builder-datastore.default \
  && hab svc load habitat/builder-jobsrv --bind datastore:builder-datastore.default

VOLUME ["/hab/svc", "/hab/cache/keys", "/hab/sup"]
EXPOSE 80 443 9631 9636 9638
ENTRYPOINT ["/usr/local/bin/hab-entrypoint.sh"]
CMD ["run"]
