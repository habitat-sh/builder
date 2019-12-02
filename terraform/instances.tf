////////////////////////////////
// Front-end Instances

provider "aws" {
  region  = var.aws_region
  profile = "habitat"
}

resource "aws_instance" "api" {
  ami           = var.aws_ami[var.aws_region]
  instance_type = var.instance_size_api
  key_name      = var.aws_key_pair
  subnet_id     = var.public_subnet_id
  count         = var.api_count

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
    private_key         = file(var.connection_private_key)
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
      "${path.module}/scripts/init_filesystem.sh",
      "${path.module}/scripts/foundation.sh",
    ]
  }

  provisioner "remote-exec" {
    inline = [
      "DD_INSTALL_ONLY=true DD_API_KEY=${var.datadog_api_key} /bin/bash -c \"$(curl -L https://raw.githubusercontent.com/DataDog/dd-agent/master/packaging/datadog-agent/source/install_agent.sh)\"",
      "sudo sed -i \"$ a tags: env:${var.env}, role:api\" /etc/dd-agent/datadog.conf",
      "sudo sed -i \"$ a use_dogstatsd: yes\" /etc/dd-agent/datadog.conf",
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
    source      = "${path.module}/files/nginx.logrotate"
    destination = "/tmp/nginx.logrotate"
  }

  provisioner "remote-exec" {
    inline = [
      "sudo cp /tmp/nginx.yaml /etc/dd-agent/conf.d/nginx.yaml",
      "sudo cp /tmp/mcache.yaml /etc/dd-agent/conf.d/mcache.yaml",
      "sudo cp /tmp/nginx.logrotate /etc/logrotate.d/nginx",
      "sudo /etc/init.d/datadog-agent start",
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
    content = templatefile(
      "${path.module}/templates/hab-sup.service.tpl",
      {
        flags = "--auto-update --peer ${join(" ", var.peers)} --channel ${var.sup_release_channel} --listen-gossip 0.0.0.0:${var.gossip_listen_port} --listen-http 0.0.0.0:${var.http_listen_port}"
        log_level = var.log_level
        enabled_features = var.enabled_features
      })
    destination = "/home/ubuntu/hab-sup.service"
  }

  provisioner "file" {
    source      = "${path.module}/files/sup_log.yml"
    destination = "/tmp/sup_log.yml"
  }

  provisioner "remote-exec" {
    inline = [
      "chmod +x /tmp/install_base_packages.sh",
      "sudo /tmp/install_base_packages.sh habitat/builder-api",
      "sudo mv /home/ubuntu/hab-sup.service /etc/systemd/system/hab-sup.service",
      "sudo mkdir -p /hab/sup/default/config",
      "sudo mv /tmp/sup_log.yml /hab/sup/default/config/log.yml",
      "sudo systemctl daemon-reload",
      "sudo systemctl start hab-sup",
      "sudo systemctl enable hab-sup",
      "sleep 10",
      "sudo hab svc load habitat/builder-memcached --group ${var.env} --strategy at-once --url ${var.bldr_url} --channel ${var.release_channel}",
      "sudo hab svc load habitat/builder-api --group ${var.env} --bind memcached:builder-memcached.${var.env} --bind jobsrv:builder-jobsrv.${var.env} --strategy at-once --url ${var.bldr_url} --channel ${var.release_channel}",
      "sudo hab svc load habitat/builder-api-proxy --group ${var.env} --bind http:builder-api.${var.env} --strategy at-once --url ${var.bldr_url} --channel ${var.release_channel}",
      "sudo hab svc load core/sumologic --group ${var.env} --strategy at-once --url ${var.bldr_url} --channel ${var.release_channel}",
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
    private_key         = file(var.connection_private_key)
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
      "${path.module}/scripts/init_filesystem.sh",
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

  provisioner "remote-exec" {
    inline = [
      "DD_INSTALL_ONLY=true DD_API_KEY=${var.datadog_api_key} /bin/bash -c \"$(curl -L https://raw.githubusercontent.com/DataDog/dd-agent/master/packaging/datadog-agent/source/install_agent.sh)\"",
      "sudo sed -i \"$ a dogstreams: /tmp/builder-scheduler.log:/etc/dd-agent/sch_log_parser.py:my_log_parser\" /etc/dd-agent/datadog.conf",
      "sudo sed -i \"$ a tags: env:${var.env}, role:jobsrv\" /etc/dd-agent/datadog.conf",
      "sudo sed -i \"$ a use_dogstatsd: yes\" /etc/dd-agent/datadog.conf",
      "sudo cp /tmp/sch_log_parser.py /etc/dd-agent/sch_log_parser.py",
      "sudo cp /tmp/builder.logrotate /etc/logrotate.d/builder",
      "sudo /etc/init.d/datadog-agent start",
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
    content = templatefile(
      "${path.module}/templates/hab-sup.service.tpl",
      {
        flags = "--auto-update --peer ${join(" ", var.peers)} --channel ${var.sup_release_channel} --listen-gossip 0.0.0.0:${var.gossip_listen_port} --listen-http 0.0.0.0:${var.http_listen_port}"
        log_level = var.log_level
        enabled_features = var.enabled_features
      })
    destination = "/home/ubuntu/hab-sup.service"
  }

  provisioner "file" {
    source      = "${path.module}/files/sup_log.yml"
    destination = "/tmp/sup_log.yml"
  }

  provisioner "remote-exec" {
    inline = [
      "chmod +x /tmp/install_base_packages.sh",
      "sudo /tmp/install_base_packages.sh habitat/builder-jobsrv",
      "sudo mv /home/ubuntu/hab-sup.service /etc/systemd/system/hab-sup.service",
      "sudo mkdir -p /hab/sup/default/config",
      "sudo mv /tmp/sup_log.yml /hab/sup/default/config/log.yml",
      "sudo systemctl daemon-reload",
      "sudo systemctl start hab-sup",
      "sudo systemctl enable hab-sup",
      "sleep 10",
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
    private_key         = file(var.connection_private_key)
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
      "${path.module}/scripts/init_filesystem.sh",
      "${path.module}/scripts/foundation.sh",
      "${path.module}/scripts/worker_bootstrap.sh",
    ]
  }

  provisioner "file" {
    source      = "${path.module}/files/builder.logrotate"
    destination = "/tmp/builder.logrotate"
  }

  provisioner "remote-exec" {
    inline = [
      "DD_INSTALL_ONLY=true DD_API_KEY=${var.datadog_api_key} /bin/bash -c \"$(curl -L https://raw.githubusercontent.com/DataDog/dd-agent/master/packaging/datadog-agent/source/install_agent.sh)\"",
      "sudo sed -i \"$ a tags: env:${var.env}, role:worker\" /etc/dd-agent/datadog.conf",
      "sudo sed -i \"$ a use_dogstatsd: yes\" /etc/dd-agent/datadog.conf",
      "sudo cp /tmp/builder.logrotate /etc/logrotate.d/builder",
      "sudo /etc/init.d/datadog-agent stop",
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
    content = templatefile(
      "${path.module}/templates/hab-sup.service.tpl",
      {
        flags = "--auto-update --peer ${join(" ", var.peers)} --channel ${var.sup_release_channel} --listen-gossip 0.0.0.0:${var.gossip_listen_port} --listen-http 0.0.0.0:${var.http_listen_port}"
        log_level = var.log_level
        enabled_features = var.enabled_features
      })
    destination = "/home/ubuntu/hab-sup.service"
  }

  provisioner "file" {
    source      = "${path.module}/files/sup_log.yml"
    destination = "/tmp/sup_log.yml"
  }

  provisioner "remote-exec" {
    inline = [
      "chmod +x /tmp/install_base_packages.sh",
      "sudo /tmp/install_base_packages.sh habitat/builder-worker",
      "sudo iptables -I DOCKER-USER -p tcp -s 10.0.0.0/24 -j DROP",
      "sudo iptables -I DOCKER-USER -p udp -s 10.0.0.0/24 -m multiport --sports 0:52,54:65535 -j DROP",
      "sudo mv /home/ubuntu/hab-sup.service /etc/systemd/system/hab-sup.service",
      "sudo mkdir -p /hab/sup/default/config",
      "sudo mv /tmp/sup_log.yml /hab/sup/default/config/log.yml",
      "sudo systemctl daemon-reload",
      "sudo systemctl start hab-sup",
      "sudo systemctl enable hab-sup",
      "sleep 10",
      "sudo hab svc load habitat/builder-worker --group ${var.env} --bind jobsrv:builder-jobsrv.${var.env} --bind depot:builder-api-proxy.${var.env} --strategy at-once --url ${var.bldr_url} --channel ${var.release_channel}",
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
    private_key         = file(var.connection_private_key)
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
    source      = "${path.module}/scripts/install_linux2_packages.sh"
    destination = "/tmp/install_linux2_packages.sh"
  }

  provisioner "remote-exec" {
    scripts = [
      "${path.module}/scripts/init_filesystem.sh",
      "${path.module}/scripts/foundation.sh",
      "${path.module}/scripts/linux2_worker_bootstrap.sh",
    ]
  }

  provisioner "file" {
    content = templatefile(
      "${path.module}/templates/hab-sup.init.tpl",
      {
        flags = "--auto-update --peer ${join(" ", var.peers)} --channel ${var.sup_release_channel} --listen-gossip 0.0.0.0:${var.gossip_listen_port} --listen-http 0.0.0.0:${var.http_listen_port}"
        log_level = var.log_level
        enabled_features = var.enabled_features
      })
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

  provisioner "remote-exec" {
    inline = [
      "sudo mkdir -p /hab/svc/builder-worker",
      "sudo mv /tmp/worker.user.toml /hab/svc/builder-worker/user.toml",
    ]
  }

  provisioner "remote-exec" {
    inline = [
      "chmod +x /tmp/install_linux2_packages.sh",
      "sudo /tmp/install_linux2_packages.sh",
      "sudo iptables -I DOCKER -p tcp -s 10.0.0.0/24 -j DROP",
      "sudo iptables -I DOCKER -p udp -s 10.0.0.0/24 -m multiport --sports 0:52,54:65535 -j DROP",
      "sudo mv /tmp/hab-sup.init /etc/init/hab-sup.conf",
      "sudo mkdir -p /hab/sup/default/config",
      "sudo mv /tmp/sup_log.yml /hab/sup/default/config/log.yml",
      "sudo service hab-sup start",
      "sleep 10",
      "sudo hab svc load habitat/builder-worker --group ${var.env} --bind jobsrv:builder-jobsrv.${var.env} --bind depot:builder-api-proxy.${var.env} --strategy at-once --url ${var.bldr_url} --channel ${var.release_channel}",
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
  // Windows_Server-2019-English-Full-ContainersLatest-2019.10.09
  ami           = "ami-0a6b38f2d62c0cc94"
  instance_type = var.instance_size_windows_worker
  key_name      = var.aws_key_pair

  // JW TODO: switch to private subnet after VPN is ready
  subnet_id = var.public_subnet_id
  count     = var.windows_worker_count

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

  user_data = templatefile(
    "${path.module}/templates/windows_worker_user_data.tpl",
    {
      environment      = var.env
      password         = var.admin_password
      flags            = "--no-color --auto-update --peer ${join(" ", var.peers)} --channel ${var.sup_release_channel} --listen-gossip 0.0.0.0:${var.gossip_listen_port} --listen-http 0.0.0.0:${var.http_listen_port}"
      bldr_url         = var.bldr_url
      channel          = var.release_channel
      enabled_features = var.enabled_features
    })

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

}
