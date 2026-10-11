/** What the Pro card charges, decided in one place so it can be tested
 *  (scripts/plans-price.test.mjs). Display only: the server picks the real
 *  Stripe Price at checkout. Amounts are euro cents. */

/** The prices the app has always shown, used when the server publishes no offer. */
export const REGULAR_CENTS = 999;
export const LOYALTY_CENTS = 499;

export type PriceOffer = { regular: number; reference: number; price: number };

/** The Halloween offer is for people who would pay the standard price today:
 *  not current subscribers, and not loyalty accounts, whose €4.99 the server
 *  never discounts a second time. `shown` is the offer to draw, if any. */
export function proPrice<T extends PriceOffer>(
  loyalty: boolean,
  proActive: boolean,
  offer: T | null,
): { shown: T | null; price: number } {
  const shown = loyalty || proActive ? null : offer;
  return {
    shown,
    price: loyalty ? LOYALTY_CENTS : (shown?.price ?? offer?.regular ?? REGULAR_CENTS),
  };
}

/** "€9.99" / "9,99 €"; whole amounts without decimals ("€0"). */
export function money(cents: number, locale: string): string {
  return new Intl.NumberFormat(locale, {
    style: "currency",
    currency: "EUR",
    minimumFractionDigits: cents % 100 === 0 ? 0 : 2,
  }).format(cents / 100);
}
