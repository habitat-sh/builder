////////////////////////////////
// Front-end Instances

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
    })
}

resource "aws_instance" "api" {
  ami           = var.aws_ami[var.aws_region]
  instance_type = var.instance_size_api
  key_name      = var.aws_key_pair
  subnet_id     = var.private_subnet_id
  count         = var.api_count
  iam_instance_profile = var.aws_instance_profile.name

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
    volume_type = "gp3"
    encrypted = true
  }

  ebs_block_device {
    device_name = "/dev/xvdf"
    volume_size = 100
    encrypted   = true
    volume_type = "gp3"
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
    scripts = [ 
      "${path.module}/scripts/monitoring.sh"
     ]
  }
  provisioner "file" {
    source      = "${path.module}/files/cloudwatch-agent-config.json"
    destination = "/opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json"
  }
  provisioner "file" {
    source      = "${path.module}/files/nginx.logrotate"
    destination = "/tmp/nginx.logrotate"
  }

  provisioner "remote-exec" {
    inline = [
      "sudo cp /tmp/nginx.logrotate /etc/logrotate.d/nginx",
      "AutomoxKey=${var.automox_api_key}",
      "/opt/aws/amazon-cloudwatch-agent/bin/amazon-cloudwatch-agent-ctl -a fetch-config -m ec2 -s -c file:/opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json",
      "sudo systemctl restart amazon-cloudwatch-agent"
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
      "sudo hab svc load habitat/builder-api --group ${var.env} --bind memcached:builder-memcached.${var.env} --binding-mode relaxed --strategy at-once --url ${var.bldr_url} --channel ${var.release_channel}",
      "sudo hab svc load habitat/builder-api-proxy --group ${var.env} --bind http:builder-api.${var.env} --strategy at-once --url ${var.bldr_url} --channel ${var.release_channel}",
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
    X-Production  = var.production
    team          = "cloudclub"
    application   = "builder"
    owner         = "chef-ops-list@progress.com"
    expiration    = "2025.12.31"
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
