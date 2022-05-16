FROM ubuntu:18.04 as build_image

# Channel from which to install Habitat-related packages
ARG CHANNEL=stable
ARG HAB_ORIGIN=habitat

# Bootstrap the installation of Habitat
RUN apt-get update && apt-get -y install curl git libczmq-dev busybox
RUN curl https://raw.githubusercontent.com/habitat-sh/habitat/main/components/hab/install.sh -o install.sh
RUN chmod a+x ./install.sh
RUN /bin/bash -c ./install.sh

# Always accept the license when we run this image.
ENV HAB_LICENSE=accept-no-persist

RUN  hab pkg install --channel="${CHANNEL}" core/hab \
  && hab pkg install --channel="${CHANNEL}" core/hab-sup \
  && hab pkg install --channel="${CHANNEL}" core/hab-launcher \
  && hab pkg install --channel="${CHANNEL}" --binlink core/hab-studio \
  && hab pkg install --channel="stable" core/busybox \
  && hab pkg binlink core/hab -d /hab/bin

RUN hab pkg binlink core/busybox --dest=/tmp/rootfs/bin

EXPOSE 80
EXPOSE 9631
EXPOSE 9632
EXPOSE 9638
EXPOSE 5566
EXPOSE 5567
EXPOSE 5568

#COPY test/builder-worker-docker/components/builder-core/bldr.env .
#COPY test/builder-worker-docker/components/builder-core/builder-github-app.pem .
#COPY test/builder-worker-docker/components/builder-core/entrypoint.sh .
#COPY test/builder-worker-docker/components/builder-core/functions.sh .
COPY bldr.env /
COPY builder-github-app.pem /
COPY entrypoint.sh /
COPY functions.sh /


RUN chmod +x entrypoint.sh

RUN hab pkg install core/git --binlink --force

RUN hab pkg install core/cacerts
ARG SSL_CERT_FILE="$(hab pkg path core/cacerts)/ssl/cert.pem"

RUN hab pkg install core/docker --binlink --force

RUN useradd hab

ENV HAB_LICENSE=accept-no-persist

ENTRYPOINT ["./entrypoint.sh"]
