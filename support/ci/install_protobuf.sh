#!/bin/sh

export HAB_LICENSE="accept-no-persist"
sudo hab pkg install --channel=LTS-2024 --binlink core/protobuf-cpp
