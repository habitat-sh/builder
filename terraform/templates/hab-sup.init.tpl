description "Habitat Supervisor"

start on filesystem or runlevel [2345]

script
    export RUST_LOG=${log_level}
    export HAB_STATS_ADDR=localhost:8125
%{ for feature in enabled_features ~}
    export HAB_FEAT_${upper(feature)}=1
%{ endfor ~}
    export SSL_CERT_FILE=$(hab pkg path core/cacerts)/ssl/cert.pem
    echo $$ > /var/run/hab-sup.pid
    echo "starting hab sup with: ${flags}" >> /var/log/hab-sup.log
    exec /bin/hab run ${flags} >> /var/log/hab-sup.log
end script

pre-start script
    echo "[`date`] hab-sup service starting" >> /var/log/hab-sup.log
end script

pre-stop script
    rm /var/run/hab-sup.pid
    echo "[`date`] hab-sup service stopping" >> /var/log/hab-sup.log
end script
