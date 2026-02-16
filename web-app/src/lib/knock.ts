import Knock from '@knocklabs/client';

let instance: Knock | null = null;

export function initKnock(publicKey: string, userId: string): Knock {
  if (instance) return instance;
  instance = new Knock(publicKey);
  instance.authenticate(userId);
  return instance;
}

export function getKnock(): Knock | null {
  return instance;
}

export function resetKnock(): void {
  instance = null;
}
