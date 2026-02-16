import { describe, it, expect, vi, beforeEach } from 'vitest';
import { registerPushToken } from '$lib/push';

describe('registerPushToken', () => {
  it('calls setChannelData with FCM channel ID and token', async () => {
    const setChannelData = vi.fn().mockResolvedValue(undefined);
    const mockKnock = {
      user: { setChannelData },
    } as any;

    await registerPushToken(mockKnock, 'user-1', 'fcm-channel-id', 'fcm-token-abc');

    expect(setChannelData).toHaveBeenCalledWith({
      channelId: 'fcm-channel-id',
      channelData: {
        tokens: ['fcm-token-abc'],
      },
    });
  });

  it('propagates errors from setChannelData', async () => {
    const setChannelData = vi.fn().mockRejectedValue(new Error('Network error'));
    const mockKnock = {
      user: { setChannelData },
    } as any;

    await expect(
      registerPushToken(mockKnock, 'user-1', 'fcm-channel-id', 'fcm-token-abc')
    ).rejects.toThrow('Network error');
  });
});
