////////////////////////////////
// Front-end Instances

provider "aws" {
  region  = var.aws_region
  profile = "habitat"
}

locals {
  # We have a few instances that run on Linux, and all should have the
  # same SystemD unit file. We declare it here to keep things DRY.
  hab_sup_service_content = templatefile(
    "${path.module}/templates/hab-sup.service.tpl",
    {
      flags            = "--auto-update --peer ${join(" ", var.peers)} --channel ${var.sup_release_channel} --listen-gossip 0.0.0.0:${var.gossip_listen_port} --listen-http 0.0.0.0:${var.http_listen_port}"
      log_level        = var.log_level
      enabled_features = var.enabled_features
    })

  # Init file for Linux kernel 2 Supervisors
  hab_sup_init_content = templatefile(
    "${path.module}/templates/hab-sup.init.tpl",
    {
      flags            = "--auto-update --peer ${join(" ", var.peers)} --channel ${var.sup_release_channel} --listen-gossip 0.0.0.0:${var.gossip_listen_port} --listen-http 0.0.0.0:${var.http_listen_port}"
      log_level        = var.log_level
      enabled_features = var.enabled_features
    })

  # Userdata for Windows workers (the only Windows Supervisors we
  # currently run)
  windows_worker_user_data_content = templatefile(
    "${path.module}/templates/windows_worker_user_data.tpl",
    {
      environment            = var.env
      password               = var.admin_password
      flags                  = "--no-color --auto-update --peer ${join(" ", var.peers)} --channel ${var.sup_release_channel} --listen-gossip 0.0.0.0:${var.gossip_listen_port} --listen-http 0.0.0.0:${var.http_listen_port}"
      bldr_url               = var.bldr_url
      worker_release_channel = var.worker_release_channel
      enabled_features       = var.enabled_features
      authorized_keys        = var.connection_public_key
      datadog_api_key        = var.datadog_api_key
    })
}

resource "aws_instance" "api" {
  ami           = var.aws_ami[var.aws_region]
  instance_type = var.instance_size_api
  key_name      = var.aws_key_pair
  subnet_id     = var.public_subnet_id
  count         = var.api_count

  lifecycle {
    ignore_changes = ["ami", "tags", "instance_type"]
  }

  vpc_security_group_ids = [
    var.aws_admin_sg,
    var.hab_sup_sg,
    aws_security_group.datastore_client.id,
    aws_security_group.gateway.id,
  ]

  connection {
    type = "ssh"
    // JW TODO: switch to private ip after VPN is ready
    host                = self.public_ip
    user                = "ubuntu"
    private_key         = var.connection_private_key
    agent               = var.connection_agent
    bastion_host        = var.bastion_host
    bastion_user        = var.bastion_user
    bastion_private_key = file(var.bastion_private_key)
  }

  root_block_device {
    volume_size = 20
  }

  ebs_block_device {
    device_name = "/dev/xvdf"
    volume_size = 100
    volume_type = "gp2"
  }

  provisioner "file" {
    source      = "${path.module}/scripts/install_base_packages.sh"
    destination = "/tmp/install_base_packages.sh"
  }

  provisioner "remote-exec" {
    scripts = [
      "${path.module}/scripts/foundation.sh",
    ]
  }

  provisioner "remote-exec" {
    inline = [
      "DD_AGENT_MAJOR_VERSION=7 DD_SITE=datadoghq.com DD_API_KEY=${var.datadog_api_key} /bin/bash -c \"$(curl -L https://s3.amazonaws.com/dd-agent/scripts/install_script.sh)\"",
      "sudo sed -i \"$ a tags:\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a  - env:${var.env}\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a  - role:api\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a use_dogstatsd: true\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a process_config: \" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a \\ enabled: true\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a logs_enabled: true\" /etc/datadog-agent/datadog.yaml",
    ]
  }

  provisioner "file" {
    source      = "${path.module}/files/nginx.yaml"
    destination = "/tmp/nginx.yaml"
  }

  provisioner "file" {
    source      = "${path.module}/files/mcache.yaml"
    destination = "/tmp/mcache.yaml"
  }

  provisioner "file" {
    source      = "${path.module}/files/syslog.yaml"
    destination = "/tmp/syslog.yaml"
  }
  provisioner "file" {
    source      = "${path.module}/files/nginx.logrotate"
    destination = "/tmp/nginx.logrotate"
  }

  provisioner "remote-exec" {
    inline = [
      "sudo mkdir /etc/datadog-agent/conf.d/syslog.d",
      "sudo cp /tmp/nginx.yaml /etc/datadog-agent/conf.d/nginx.d/conf.yaml",
      "sudo cp /tmp/mcache.yaml /etc/datadog-agent/conf.d/mcache.d/conf.yaml",
      "sudo cp /tmp/syslog.yaml /etc/datadog-agent/conf.d/syslog.d/conf.yaml",
      "sudo cp /tmp/nginx.logrotate /etc/logrotate.d/nginx",
      "sudo usermod -a -G adm dd-agent",
      "sudo systemctl restart datadog-agent",
      "sudo systemctl enable datadog-agent",
    ]
  }

  provisioner "file" {
    source      = "${path.module}/files/sumocollector.service"
    destination = "/tmp/sumocollector.service"
  }

  provisioner "remote-exec" {
    inline = [
      "sudo mv /tmp/sumocollector.service /etc/systemd/system/sumocollector.service",
      "sudo systemctl enable /etc/systemd/system/sumocollector.service",
      "sudo systemctl start sumocollector.service",
    ]
  }

  provisioner "file" {
    content = local.hab_sup_service_content
    destination = "/home/ubuntu/hab-sup.service"
  }

  provisioner "file" {
    source      = "${path.module}/files/sup_log.yml"
    destination = "/tmp/sup_log.yml"
  }

  provisioner "remote-exec" {
    inline = [
      "chmod +x /tmp/install_base_packages.sh",
      "sudo /tmp/install_base_packages.sh -s habitat/builder-api",
      "sudo mv /home/ubuntu/hab-sup.service /etc/systemd/system/hab-sup.service",
      "sudo mkdir -p /hab/sup/default/config",
      "sudo mv /tmp/sup_log.yml /hab/sup/default/config/log.yml",
      "sudo systemctl daemon-reload",
      "sudo systemctl start hab-sup",
      "sudo systemctl enable hab-sup",
      "until sudo hab svc status; do sleep 5; done",
      "echo \"Supervisor is up. Sleeping 120s to allow for auto upgrade.\"",
      "sleep 120",
      "sudo hab svc load habitat/builder-memcached --group ${var.env} --strategy at-once --url ${var.bldr_url} --channel ${var.release_channel}",
      "sudo hab svc load habitat/builder-api --group ${var.env} --bind memcached:builder-memcached.${var.env} --bind jobsrv:builder-jobsrv.${var.env} --binding-mode relaxed --strategy at-once --url ${var.bldr_url} --channel ${var.release_channel}",
      "sudo hab svc load habitat/builder-api-proxy --group ${var.env} --bind http:builder-api.${var.env} --strategy at-once --url ${var.bldr_url} --channel ${var.release_channel}",
      "sudo hab svc load core/sumologic --group ${var.env} --strategy at-once --url ${var.bldr_url} --channel ${var.release_channel}",
    ]
  }

  provisioner "file" {
    source      = "${path.module}/files/db_connect.sh"
    destination = "/home/ubuntu/db_connect.sh"
  }

  provisioner "remote-exec" {
    inline = [
      "chmod +x /home/ubuntu/db_connect.sh",
    ]
  }

  tags = {
    Name          = "builder-api-${count.index}"
    X-Contact     = "The Habitat Maintainers <humans@habitat.sh>"
    X-Environment = var.env
    X-Application = "builder"
    X-ManagedBy   = "Terraform"
  }
}

////////////////////////////////
// Back-end Instances

resource "aws_instance" "jobsrv" {
  ami           = var.aws_ami[var.aws_region]
  instance_type = var.instance_size_jobsrv
  key_name      = var.aws_key_pair

  // JW TODO: switch to private subnet after VPN is ready
  subnet_id = var.public_subnet_id
  count     = 1

  lifecycle {
    ignore_changes = ["ami", "tags", "instance_type"]
  }

  vpc_security_group_ids = [
    var.aws_admin_sg,
    var.hab_sup_sg,
    aws_security_group.datastore_client.id,
    aws_security_group.jobsrv.id,
    aws_security_group.service.id,
  ]

  connection {
    type = "ssh"
    // JW TODO: switch to private ip after VPN is ready
    host                = self.public_ip
    user                = "ubuntu"
    private_key         = var.connection_private_key
    agent               = var.connection_agent
    bastion_host        = var.bastion_host
    bastion_user        = var.bastion_user
    bastion_private_key = file(var.bastion_private_key)
  }

  root_block_device {
    volume_size = 20
  }

  ebs_block_device {
    device_name = "/dev/xvdf"
    volume_size = 100
    volume_type = "gp2"
  }

  provisioner "file" {
    source      = "${path.module}/scripts/install_base_packages.sh"
    destination = "/tmp/install_base_packages.sh"
  }

  provisioner "remote-exec" {
    scripts = [
      "${path.module}/scripts/foundation.sh",
    ]
  }

  provisioner "file" {
    content     = data.template_file.sch_log_parser.rendered
    destination = "/tmp/sch_log_parser.py"
  }

  provisioner "file" {
    source      = "${path.module}/files/builder.logrotate"
    destination = "/tmp/builder.logrotate"
  }

  provisioner "file" {
    source      = "${path.module}/files/syslog.yaml"
    destination = "/tmp/syslog.yaml"
  }

  provisioner "file" {
    source      = "${path.module}/files/scheduler.yaml"
    destination = "/tmp/scheduler.yaml"
  }


  provisioner "remote-exec" {
    inline = [
      "DD_AGENT_MAJOR_VERSION=7 DD_SITE=datadoghq.com DD_API_KEY=${var.datadog_api_key} /bin/bash -c \"$(curl -L https://s3.amazonaws.com/dd-agent/scripts/install_script.sh)\"",
      "sudo sed -i \"$ a tags:\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a  - env:${var.env}\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a  - role:jobsrv\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a use_dogstatsd: true\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a process_config: \" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a \\ enabled: true\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a logs_enabled: true\" /etc/datadog-agent/datadog.yaml",
      "sudo mkdir /etc/datadog-agent/conf.d/syslog.d",
      "sudo mkdir /etc/datadog-agent/conf.d/scheduler.d",
      "sudo cp /tmp/syslog.yaml /etc/datadog-agent/conf.d/syslog.d/conf.yaml",
      "sudo cp /tmp/scheduler.yaml /etc/datadog-agent/conf.d/scheduler.d/conf.yaml",
      "sudo cp /tmp/sch_log_parser.py /etc/dd-agent/sch_log_parser.py",
      "sudo cp /tmp/builder.logrotate /etc/logrotate.d/builder",
      "sudo usermod -a -G adm dd-agent",
      "sudo systemctl restart datadog-agent",
      "sudo systemctl enable datadog-agent",
    ]
  }

  provisioner "file" {
    source      = "${path.module}/files/sumocollector.service"
    destination = "/tmp/sumocollector.service"
  }

  provisioner "remote-exec" {
    inline = [
      "sudo mv /tmp/sumocollector.service /etc/systemd/system/sumocollector.service",
      "sudo systemctl enable /etc/systemd/system/sumocollector.service",
      "sudo systemctl start sumocollector.service",
    ]
  }

  provisioner "file" {
    content = local.hab_sup_service_content
    destination = "/home/ubuntu/hab-sup.service"
  }

  provisioner "file" {
    source      = "${path.module}/files/sup_log.yml"
    destination = "/tmp/sup_log.yml"
  }

  provisioner "remote-exec" {
    inline = [
      "chmod +x /tmp/install_base_packages.sh",
      "sudo /tmp/install_base_packages.sh -s habitat/builder-jobsrv",
      "sudo mv /home/ubuntu/hab-sup.service /etc/systemd/system/hab-sup.service",
      "sudo mkdir -p /hab/sup/default/config",
      "sudo mv /tmp/sup_log.yml /hab/sup/default/config/log.yml",
      "sudo systemctl daemon-reload",
      "sudo systemctl start hab-sup",
      "sudo systemctl enable hab-sup",
      "until sudo hab svc status; do sleep 10; done",
      "echo \"Supervisor is up. Sleeping 120s to allow for auto upgrade.\"",
      "sleep 120",
      "sudo hab svc load habitat/builder-jobsrv --group ${var.env} --strategy at-once --url ${var.bldr_url} --channel ${var.release_channel}",
      "sudo hab svc load core/sumologic --group ${var.env} --strategy at-once --url ${var.bldr_url} --channel ${var.release_channel}",
    ]
  }

  tags = {
    Name          = "builder-jobsrv-${count.index}"
    X-Contact     = "The Habitat Maintainers <humans@habitat.sh>"
    X-Environment = var.env
    X-Application = "builder"
    X-ManagedBy   = "Terraform"
  }
}

resource "aws_instance" "worker" {
  ami           = var.aws_ami[var.aws_region]
  instance_type = var.instance_size_worker
  key_name      = var.aws_key_pair

  // JW TODO: switch to private subnet after VPN is ready
  subnet_id = var.public_subnet_id
  count     = var.jobsrv_worker_count

  lifecycle {
    ignore_changes = ["ami", "tags", "instance_type"]
  }

  vpc_security_group_ids = [
    var.aws_admin_sg,
    var.hab_sup_sg,
    aws_security_group.jobsrv_client.id,
    aws_security_group.worker.id,
  ]
  
  connection {
    type = "ssh"
    // JW TODO: switch to private ip after VPN is ready
    host                = self.public_ip
    user                = "ubuntu"
    private_key         = var.connection_private_key
    agent               = var.connection_agent
    bastion_host        = var.bastion_host
    bastion_user        = var.bastion_user
    bastion_private_key = file(var.bastion_private_key)
  }

  root_block_device {
    volume_size = 20
  }

  ebs_block_device {
    device_name = "/dev/xvdf"
    volume_size = 250
    volume_type = "gp2"
  }


  provisioner "file" {
    source      = "${path.module}/scripts/install_base_packages.sh"
    destination = "/tmp/install_base_packages.sh"
  }

  provisioner "remote-exec" {
    scripts = [
      "${path.module}/scripts/foundation.sh",
      "${path.module}/scripts/worker_bootstrap.sh",
    ]
  }
  
  provisioner "file" {
    source      = "${path.module}/files/builder.logrotate"
    destination = "/tmp/builder.logrotate"
  }

  provisioner "file" {
    source      = "${path.module}/files/docker.yaml"
    destination = "/tmp/docker.yaml"
  }

  provisioner "file" {
    source      = "${path.module}/files/syslog.yaml"
    destination = "/tmp/syslog.yaml"
  }

  provisioner "file" {
    source      = "${path.module}/files/worker.yaml"
    destination = "/tmp/worker.yaml"
  }

  provisioner "remote-exec" {
    inline = [
      "DD_AGENT_MAJOR_VERSION=7 DD_SITE=datadoghq.com DD_API_KEY=${var.datadog_api_key} /bin/bash -c \"$(curl -L https://s3.amazonaws.com/dd-agent/scripts/install_script.sh)\"",
      "sudo sed -i \"$ a tags:\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a  - env:${var.env}\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a  - role:worker\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a use_dogstatsd: true\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a process_config: \" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a \\ enabled: 'true'\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a logs_enabled: true\" /etc/datadog-agent/datadog.yaml",
      "sudo mv /tmp/docker.yaml /etc/datadog-agent/conf.d/docker.d/conf.yaml",
      "sudo mkdir /etc/datadog-agent/conf.d/syslog.d",
      "sudo mkdir /etc/datadog-agent/conf.d/worker.d",
      "sudo mv /tmp/worker.yaml /etc/datadog-agent/conf.d/worker.d/conf.yaml",
      "sudo cp /tmp/syslog.yaml /etc/datadog-agent/conf.d/syslog.d/conf.yaml",
      "sudo usermod -a -G docker dd-agent",
      "sudo usermod -a -G adm dd-agent",
      "sudo cp /tmp/builder.logrotate /etc/logrotate.d/builder",
      "sudo systemctl restart datadog-agent",
      "sudo systemctl enable datadog-agent",
    ]
  }

  provisioner "remote-exec" {
    inline = [
      "sudo mkdir -p /home/ubuntu/.hab/accepted-licenses",
      "sudo mkdir -p /home/hab/.hab/accepted-licenses",
      "sudo mkdir -p /hab/accepted-licenses",
      "sudo touch /home/ubuntu/.hab/accepted-licenses/habitat",
      "sudo touch /home/hab/.hab/accepted-licenses/habitat",
      "sudo touch /hab/accepted-licenses/habitat",
    ]
  }

  provisioner "file" {
    source      = "${path.module}/files/sumocollector.service"
    destination = "/tmp/sumocollector.service"
  }

  provisioner "remote-exec" {
    inline = [
      "sudo mv /tmp/sumocollector.service /etc/systemd/system/sumocollector.service",
      "sudo systemctl enable /etc/systemd/system/sumocollector.service",
      "sudo systemctl start sumocollector.service",
    ]
  }

  provisioner "file" {
    content = local.hab_sup_service_content
    destination = "/home/ubuntu/hab-sup.service"
  }

  provisioner "file" {
    source      = "${path.module}/files/sup_log.yml"
    destination = "/tmp/sup_log.yml"
  }

  provisioner "remote-exec" {
    inline = [
      "chmod +x /tmp/install_base_packages.sh",
      "sudo /tmp/install_base_packages.sh -s habitat/builder-worker",
      "sudo iptables -I DOCKER-USER -p tcp -s 10.0.0.0/24 -j DROP",
      "sudo iptables -I DOCKER-USER -p udp -s 10.0.0.0/24 -m multiport --sports 0:52,54:65535 -j DROP",
      "sudo mv /home/ubuntu/hab-sup.service /etc/systemd/system/hab-sup.service",
      "sudo mkdir -p /hab/sup/default/config",
      "sudo mv /tmp/sup_log.yml /hab/sup/default/config/log.yml",
      "sudo systemctl daemon-reload",
      "sudo systemctl start hab-sup",
      "sudo systemctl enable hab-sup",
      "until sudo hab svc status; do sleep 10; done",
      "echo \"Supervisor is up. Sleeping 120s to allow for auto upgrade.\"",
      "sleep 120",
      "sudo hab svc load habitat/builder-worker --group ${var.env} --bind jobsrv:builder-jobsrv.${var.env} --bind depot:builder-api-proxy.${var.env} --strategy at-once --url ${var.bldr_url} --channel ${var.worker_release_channel}",
      "sudo hab svc load core/sumologic --group ${var.env} --strategy at-once --url ${var.bldr_url} --channel ${var.release_channel}",
    ]
  }

  tags = {
    Name          = "builder-worker-${count.index}"
    X-Contact     = "The Habitat Maintainers <humans@habitat.sh>"
    X-Environment = var.env
    X-Application = "builder"
    X-ManagedBy   = "Terraform"
  }
}

resource "aws_instance" "linux2-worker" {
  ami           = "ami-0ea790e761025f9ce" // Ubuntu 14.04
  instance_type = var.instance_size_linux2_worker
  key_name      = var.aws_key_pair

  // JW TODO: switch to private subnet after VPN is ready
  subnet_id = var.public_subnet_id
  count     = var.linux2_worker_count

  lifecycle {
    ignore_changes = ["ami", "tags", "instance_type"]
  }

  vpc_security_group_ids = [
    var.aws_admin_sg,
    var.hab_sup_sg,
    aws_security_group.jobsrv_client.id,
    aws_security_group.worker.id,
  ]

  connection {
    type = "ssh"
    // JW TODO: switch to private ip after VPN is ready
    host                = self.public_ip
    user                = "ubuntu"
    private_key         = var.connection_private_key
    agent               = var.connection_agent
    bastion_host        = var.bastion_host
    bastion_user        = var.bastion_user
    bastion_private_key = file(var.bastion_private_key)
  }

  root_block_device {
    volume_size = 20
  }

  ebs_block_device {
    device_name = "/dev/xvdf"
    volume_size = 250
    volume_type = "gp2"
  }

  provisioner "file" {
    source      = "${path.module}/scripts/install_base_packages.sh"
    destination = "/tmp/install_base_packages.sh"
  }

  provisioner "remote-exec" {
    scripts = [
      "${path.module}/scripts/foundation.sh",
      "${path.module}/scripts/linux2_worker_bootstrap.sh",
    ]
  }

  provisioner "file" {
    content = local.hab_sup_init_content
    destination = "/tmp/hab-sup.init"
  }

  provisioner "file" {
    source      = "${path.module}/files/sup_log.yml"
    destination = "/tmp/sup_log.yml"
  }

  provisioner "file" {
    source      = "${path.module}/files/linux2-worker.user.toml"
    destination = "/tmp/worker.user.toml"
  }

  provisioner "file" {
    source      = "${path.module}/files/docker.yaml"
    destination = "/tmp/docker.yaml"
  }

  provisioner "file" {
    source      = "${path.module}/files/syslog.yaml"
    destination = "/tmp/syslog.yaml"
  }

  provisioner "file" {
    source      = "${path.module}/files/worker.yaml"
    destination = "/tmp/worker.yaml"
  }

  provisioner "remote-exec" {
    inline = [
      "sudo mkdir -p /hab/svc/builder-worker",
      "sudo mv /tmp/worker.user.toml /hab/svc/builder-worker/user.toml",
    ]
  }

  provisioner "remote-exec" {
    inline = [
      "chmod +x /tmp/install_base_packages.sh",
      "sudo /tmp/install_base_packages.sh -t x86_64-linux-kernel2",
      "sudo iptables -I DOCKER -p tcp -s 10.0.0.0/24 -j DROP",
      "sudo iptables -I DOCKER -p udp -s 10.0.0.0/24 -m multiport --sports 0:52,54:65535 -j DROP",
      "sudo mv /tmp/hab-sup.init /etc/init/hab-sup.conf",
      "sudo mkdir -p /hab/sup/default/config",
      "sudo mv /tmp/sup_log.yml /hab/sup/default/config/log.yml",
      "sudo service hab-sup start",
      "until sudo hab svc status; do sleep 10; done",
      "echo \"Supervisor is up. Sleeping 120s to allow for auto upgrade.\"",
      "sleep 120",
      "sudo hab svc load habitat/builder-worker --group ${var.env} --bind jobsrv:builder-jobsrv.${var.env} --bind depot:builder-api-proxy.${var.env} --strategy at-once --url ${var.bldr_url} --channel ${var.worker_release_channel}",
    ]
  }

  provisioner "remote-exec" {
    inline = [
      "DD_AGENT_MAJOR_VERSION=7 DD_SITE=datadoghq.com DD_API_KEY=${var.datadog_api_key} /bin/bash -c \"$(curl -L https://s3.amazonaws.com/dd-agent/scripts/install_script.sh)\"",
      "sudo sed -i \"$ a tags:\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a  - env:${var.env}\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a  - role:worker\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a use_dogstatsd: true\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a process_config: \" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a \\ enabled: 'true'\" /etc/datadog-agent/datadog.yaml",
      "sudo sed -i \"$ a logs_enabled: true\" /etc/datadog-agent/datadog.yaml",
      "sudo mv /tmp/docker.yaml /etc/datadog-agent/conf.d/docker.d/conf.yaml",
      "sudo mkdir /etc/datadog-agent/conf.d/worker.d",
      "sudo mkdir /etc/datadog-agent/conf.d/syslog.d",
      "sudo cp /tmp/syslog.yaml /etc/datadog-agent/conf.d/syslog.d/conf.yaml",
      "sudo mv /tmp/worker.yaml /etc/datadog-agent/conf.d/worker.d/conf.yaml",
      "sudo usermod -a -G adm dd-agent",
      "sudo usermod -a -G docker dd-agent",
      "sudo cp /tmp/builder.logrotate /etc/logrotate.d/builder",
      "sudo /etc/init.d/datadog-agent restart",
      "sudo update-rc.d datadog-agent defaults",
    ]
  }

  tags = {
    Name          = "builder-linux2-worker-${count.index}"
    X-Contact     = "The Habitat Maintainers <humans@habitat.sh>"
    X-Environment = var.env
    X-Application = "builder"
    X-ManagedBy   = "Terraform"
  }
}

resource "aws_instance" "windows-worker" {
  // Windows Server 2019 English Core with Containers 2021-05-12 
  ami           = "ami-0613a489ef66dc885"
  instance_type = var.instance_size_windows_worker
  key_name      = var.aws_key_pair

  // JW TODO: switch to private subnet after VPN is ready
  subnet_id = var.public_subnet_id
  count     = var.windows_worker_count

  lifecycle {
    ignore_changes = ["ami", "tags", "user_data", "instance_type"]
  }

  vpc_security_group_ids = [
    var.aws_admin_sg,
    var.hab_sup_sg,
    aws_security_group.jobsrv_client.id,
    aws_security_group.worker.id,
  ]

  connection {
    host                = coalesce(self.public_ip, self.private_ip)
    type                = "winrm"
    user                = "Administrator"
    password            = var.admin_password
    bastion_host        = var.bastion_host
    bastion_user        = var.bastion_user
    bastion_private_key = file(var.bastion_private_key)
  }

  root_block_device {
    volume_size = "100"
  }

  user_data = local.windows_worker_user_data_content

  tags = {
    Name          = "builder-windows-worker-${count.index}"
    X-Contact     = "The Habitat Maintainers <humans@habitat.sh>"
    X-Environment = var.env
    X-Application = "builder"
    X-ManagedBy   = "Terraform"
  }
}

////////////////////////////////
// Template Files

data "template_file" "sch_log_parser" {
  template = file("${path.module}/templates/sch_log_parser.py")

  vars = {
    bldr_url = var.bldr_url
  }
}

data "template_file" "sumo_sources_worker" {
  template = file("${path.module}/templates/sumo_sources_local.json")

  vars = {
    name     = var.env
    category = "${var.env}/worker"
    path     = "/tmp/builder-worker.log"
  }
}

data "template_file" "sumo_sources_syslog" {
  template = file("${path.module}/templates/sumo_sources_syslog.json")

  vars = {
    name     = "${var.env}-Syslog"
    category = "${var.env}/syslog"
  }
}
