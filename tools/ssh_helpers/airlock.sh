#!/usr/bin/env bash
# shellcheck disable=SC2013,SC2029

environment=${1}

airlock_interface="ens4"
ns_dir="/hab/svc/builder-worker/data/network/airlock-ns"

for worker in $(grep "Host ${environment}-builder-worker" ~/.ssh/config | awk '{print $2}' | sort); do
    echo "Worker ${worker}"
    if ssh "$worker" sudo test -d "${ns_dir}"; then
      # The Airlock network namespace is created
      output="$(ssh "$worker" sudo nsenter --user="$ns_dir/userns" --net="$ns_dir/netns" ip address show dev "$airlock_interface" | grep 'inet ')"
      if [ -n "$output" ]; then
          # The Airlock interface is present in the network namespace--which is
          # what we are expecting
          echo "Network interface $airlock_interface in network namespace $ns_dir:"
          echo "${output}"
          echo "${worker} ${airlock_interface} => OK"
      else
          echo ">>> The Airlock interface isn't in the network namespace $ns_dir."
          echo ">>>"
          echo ">>> This is an issue."
          echo ">>>"
          echo ">>> Consider removing the Airlock network namespace with:"
          echo ">>>"
          echo ">>>     sudo hab pkg exec core/airlock airlock netns destroy --ns-dir $ns_dir"
          echo ">>>"
          echo ">>> And restarting the worker with:"
          echo ">>>"
          echo ">>>     sudo systemctl restart hab-sup"
          echo ">>>"
      fi
    else
      # The Airlock network namesapce has not been created
      output="$(ssh "$worker" sudo ip address show dev "$airlock_interface" | grep 'inet ')"
      if [ -n "$output" ]; then
        echo ">>> The Airlock network namespace $ns_dir isn't present,"
        echo ">>> but the interface $airlock_interface exists on the host with an IP address."
        echo ">>> This most likely means that the worker service has not yet booted up successfully."
        echo ">>>"
        echo ">>> Network interface details in root network namespace:"
        echo ">>>"
        echo ">>> ${output}"
        echo ">>>"
      else
        echo ">>> The Airlock network namespace $ns_dir isn't present"
        echo ">>> and the interface $airlock_interface does not exist on the host."
        echo ">>> This most likely means that the extra network interface has not been attached to the host."
        echo ">>>"
        echo ">>> This is an issue."
        echo ">>>"
      fi
    fi
    echo "---"
done
