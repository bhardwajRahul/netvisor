# Scanopy

<p align="left">
  <img src="./media/logo.png" width="100" alt="Scanopy Logo">
</p>

**Network documentation, without the drawing.**

Scanopy replaces manual network diagrams with a continuously maintained model of what's actually running. A single daemon scans on a schedule and produces four views from each scan: L2 (physical), L3 (logical), workloads, and applications. Unlike diagrams drawn in draw.io that go stale the week they're saved, or IaC state that misses drift and resources provisioned outside the pipeline, Scanopy reflects the current state of your infrastructure. Export as SVG, Mermaid, or Confluence; embed live maps; or feed the model into your existing source of truth.

![Docker Pulls](https://img.shields.io/docker/pulls/mayanayza/netvisor-server?style=for-the-badge&logo=docker)  ![Github Stars](https://img.shields.io/github/stars/scanopy/scanopy?style=for-the-badge&logo=github
)<br>
![GitHub release](https://img.shields.io/github/v/release/scanopy/scanopy?style=for-the-badge) ![License](https://img.shields.io/github/license/scanopy/scanopy?style=for-the-badge)<br>
![Daemon image size](https://img.shields.io/docker/image-size/mayanayza/scanopy-daemon?style=for-the-badge&label=Daemon%20image%20size) ![Server image size](https://img.shields.io/docker/image-size/mayanayza/scanopy-server?style=for-the-badge&label=Server%20image%20size
)<br>
![Daemon](https://img.shields.io/github/actions/workflow/status/scanopy/scanopy/daemon-ci.yml?label=daemon-ci&style=for-the-badge)  ![Server](https://img.shields.io/github/actions/workflow/status/scanopy/scanopy/server-ci.yml?label=server-ci&style=for-the-badge)  ![UI](https://img.shields.io/github/actions/workflow/status/scanopy/scanopy/ui-ci.yml?label=ui-ci&style=for-the-badge)<br>
[![Discord](https://img.shields.io/discord/1432872786828726392?logo=discord&label=discord&labelColor=white&color=7289da&style=for-the-badge)](https://discord.gg/b7ffQr8AcZ) [![Translations](https://img.shields.io/weblate/progress/scanopy?style=for-the-badge&logo=weblate)](https://hosted.weblate.org/engage/scanopy/)

> 💡 **Prefer not to self-host?** [Get a free trial](https://scanopy.net?utm_source=github&utm_medium=readme&utm_campaign=cloud_trial) of Scanopy Cloud

<table>
  <tr>
    <td width="50%" valign="top">
      <img src="./media/l2.png" alt="L2 view" />
      <p align="center"><strong>L2 (Physical)</strong><br/><sub>Every switch, every port, every link.</sub></p>
    </td>
    <td width="50%" valign="top">
      <img src="./media/l3.png" alt="L3 view" />
      <p align="center"><strong>L3 (Logical)</strong><br/><sub>Subnets and how hosts connect across them.</sub></p>
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <img src="./media/wl.png" alt="Workloads view" />
      <p align="center"><strong>Workloads</strong><br/><sub>Bare metal to hypervisors to containers.</sub></p>
    </td>
    <td width="50%" valign="top">
      <img src="./media/app.png" alt="Applications view" />
      <p align="center"><strong>Applications</strong><br/><sub>Services and their dependencies, grouped by application.</sub></p>
    </td>
  </tr>
</table>

## ✨ Features

- **Automatic discovery**: Maps hosts and services by scanning the network. One scanner, no per-device agents.
- **230+ service definitions**: Auto-detects databases, web servers, containers, network infrastructure, and enterprise applications.
- **Four views from one scan**: L2 (physical), L3 (logical), workloads, and application dependencies.
- **Distributed scanning**: Deploy daemons across segments to map multi-site and multi-VLAN topologies.
- **Docker & SNMP integration**: Native discovery for containerized services and network hardware.
- **Scheduled rescans**: Documentation stays current as infrastructure changes.
- **Multi-user + RBAC**: Organization management, role-based access, and shareable live views for teammates or external stakeholders.

## 🎯 Perfect For

- **Platform & DevOps teams**: Trace service dependencies without APM. Map containers, VMs, and hardware in one model.
- **Network engineers**: Multi-VLAN, multi-site topology diagrams derived from SNMP, LLDP, and ARP. No manual drawing.
- **IT operations**: Keep inventory, topology, and dependencies current across teams and sites.
- **MSPs**: Per-client documentation with shareable live views.
- **Home labs**: Document your infrastructure without opening draw.io.

## 📋 Licensing
**Self-hosted ([AGPL-3.0](LICENSE.md)):** Free for all use. Requires source disclosure for network services and copyleft compliance.   
**Self-hosted ([Commercial license](COMMERCIAL-LICENSE.md)):** For those who cannot comply with AGPL-3.0 terms. Contact licensing@scanopy.net  
**Hosted Solution:** **[Scanopy Cloud](https://scanopy.net?utm_source=github&utm_medium=readme&utm_campaign=cloud_trial)** subscription for zero infrastructure management  

## 🚀 Quick Start for Self Hosting

**Docker Compose**

```bash
curl -O https://raw.githubusercontent.com/scanopy/scanopy/refs/heads/main/docker-compose.yml
docker compose up -d
```

**Proxmox**

Use this [helper script](https://community-scripts.github.io/ProxmoxVE/scripts?id=scanopy) to create a Scanopy LXC.

**Unraid**

Available as an Unraid community app.

> 💡 **Prefer not to self-host?** [Get a free trial](https://scanopy.net?utm_source=github&utm_medium=readme&utm_campaign=cloud_trial) of Scanopy Cloud

---

Access the UI at `http://<your-server-ip>:60072`, create your account, and wait for the first discovery to complete.

For detailed setup options and configuration, see the [Installation Guide](https://scanopy.net/docs/server-installation?utm_source=github&utm_medium=readme&utm_campaign=docs).

## 📚 Documentation + API

**[scanopy.net/docs](https://scanopy.net/docs?utm_source=github&utm_medium=readme&utm_campaign=docs)**

## 🚀 Demo

**[demo.scanopy.net](https://demo.scanopy.net/?utm_source=github&utm_medium=readme&utm_campaign=demo)**. Hosted demo app with a sample dataset. Try the full UI without installing anything.

## 🤝 Contributing

We welcome contributions! See our [contributing guide](contributing.md) for details.

Great first contributions:
- [Adding service definitions](contributing.md#adding-service-definitions)
- [Translating Scanopy](https://hosted.weblate.org/engage/scanopy/) into your language

## 💬 Community & Support

- **Discord**: [Join our Discord](https://discord.gg/b7ffQr8AcZ) for help and discussions
- **Issues**: [Report bugs or request features](https://github.com/scanopy/scanopy/issues/new)
- **Discussions**: [GitHub Discussions](https://github.com/scanopy/scanopy/discussions)

---
**Translations powered by Weblate**

**Built with ❤️ in NYC**
