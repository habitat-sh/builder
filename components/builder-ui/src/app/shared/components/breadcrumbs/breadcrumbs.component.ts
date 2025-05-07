import { Component, Input, OnChanges, SimpleChanges } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';

@Component({
  selector: 'app-breadcrumbs',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './breadcrumbs.component.html',
  styleUrls: ['./breadcrumbs.component.scss']
})
export class BreadcrumbsComponent implements OnChanges {
  @Input() items: BreadcrumbItem[] = [];
  @Input() packageIdent: PackageIdent | null = null;

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['packageIdent'] && this.packageIdent) {
      // Convert package identifier to breadcrumb items
      this.generateBreadcrumbsFromPackageIdent();
    }
  }

  private generateBreadcrumbsFromPackageIdent(): void {
    if (!this.packageIdent) return;

    const items: BreadcrumbItem[] = [];
    const { origin, name, version, release } = this.packageIdent;

    // Origin item
    items.push({
      label: origin,
      routerLink: ['/origins', origin]
    });

    // Package name item
    if (name) {
      items.push({
        label: name,
        routerLink: ['/pkgs', origin, name]
      });
    }

    // Package version item
    if (name && version) {
      items.push({
        label: version,
        routerLink: ['/pkgs', origin, name, version]
      });
    }

    // Package release item
    if (name && version && release) {
      items.push({
        label: release,
        routerLink: ['/pkgs', origin, name, version, release]
      });
    }

    this.items = items;
  }
}

export interface BreadcrumbItem {
  label: string;
  routerLink?: any[];
  url?: string;
  icon?: string;
}

export interface PackageIdent {
  origin: string;
  name?: string;
  version?: string;
  release?: string;
}
