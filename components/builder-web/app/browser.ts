import * as cookies from 'js-cookie';
import config from './config';

export class Browser {

  static get cookieDomain() {
    return config.cookie_domain || this.currentHostname;
  }

  static get currentHostname() {
    return location.hostname;
  }

  static getCookie(key) {
    return cookies.get(key);
  }

  static redirect(url) {
    window.location.href = url;
  }

  static openInTab(url) {
    window.open(url, '_blank');
  }

  static removeCookie(key) {
    cookies.remove(key, { domain: this.cookieDomain });
  }

  static setCookie(key, value) {
    return cookies.set(key, value, {
      domain: this.cookieDomain,
      secure: window.location.protocol === 'https'
    });
  }
}
