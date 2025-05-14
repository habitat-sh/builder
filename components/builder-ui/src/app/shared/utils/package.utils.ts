import { Package, PackageIdent } from '../models/package.model';

/**
 * Formats a package or package identifier as a string
 * Example: "core/redis/1.2.3/20250514121345"
 */
export function packageString(pkg: Package | PackageIdent): string {
  if (!pkg) {
    return '';
  }
  
  // Handle both Package and PackageIdent objects
  const ident = 'ident' in pkg ? pkg.ident : pkg;
  
  let result = `${ident.origin}/${ident.name}`;
  
  if (ident.version) {
    result += `/${ident.version}`;
    
    if (ident.release) {
      result += `/${ident.release}`;
    }
  }
  
  return result;
}

/**
 * Get the route path for a package
 */
export function packageRoutePath(pkg: Package | PackageIdent): string[] {
  if (!pkg) {
    return [];
  }
  
  // Handle both Package and PackageIdent objects
  const ident = 'ident' in pkg ? pkg.ident : pkg;
  
  const path = ['/pkgs', ident.origin, ident.name];
  
  if (ident.version) {
    path.push(ident.version);
    
    if (ident.release) {
      path.push(ident.release);
    }
  } else {
    path.push('latest');
  }
  
  return path;
}
