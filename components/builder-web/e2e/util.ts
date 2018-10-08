import { ClientFunction } from 'testcafe';

export const getCookie = ClientFunction((key) => {
  return document.cookie.match(new RegExp(`${key}=([A-Za-z0-9-]+);`))[1];
});
