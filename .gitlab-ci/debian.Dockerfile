FROM debian:forky-slim

RUN export DEBIAN_FRONTEND=noninteractive \
   && apt-get -y update \
   && apt-get -y install --no-install-recommends wget ca-certificates gnupg eatmydata \
   && eatmydata apt-get -y update \
   && eatmydata apt-get -y dist-upgrade \
   && cd /home/user/app \
   && eatmydata apt-get --no-install-recommends -y build-dep . \
   && apt-get -y install rustup xauth xvfb \
   && eatmydata apt-get clean

