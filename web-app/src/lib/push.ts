import type Knock from '@knocklabs/client';

export async function registerServiceWorker(): Promise<ServiceWorkerRegistration | null> {
  if (!('serviceWorker' in navigator)) return null;
  return navigator.serviceWorker.register('/sw.js');
}

export async function requestPushPermission(): Promise<boolean> {
  if (!('Notification' in window)) return false;
  const result = await Notification.requestPermission();
  return result === 'granted';
}

export async function registerPushToken(
  knock: Knock,
  userId: string,
  channelId: string,
  fcmToken: string
): Promise<void> {
  await knock.user.setChannelData({
    channelId,
    channelData: {
      tokens: [fcmToken],
    },
  });
}
