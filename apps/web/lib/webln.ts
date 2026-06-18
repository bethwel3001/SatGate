declare global {
  interface Window {
    webln?: {
      enable: () => Promise<void>;
      sendPayment: (paymentRequest: string) => Promise<unknown>;
    };
  }
}

export async function payWithWebLn(paymentRequest: string) {
  if (!window.webln) {
    return false;
  }

  try {
    await window.webln.enable();
    await window.webln.sendPayment(paymentRequest);
    return true;
  } catch (e) {
    console.error("WebLN payment failed", e);
    return false;
  }
}
