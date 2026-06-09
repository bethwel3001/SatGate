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
    throw new Error("WebLN wallet not found");
  }

  await window.webln.enable();
  await window.webln.sendPayment(paymentRequest);
}
