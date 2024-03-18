const { SlashCommandBuilder, EmbedBuilder, ActionRowBuilder, ButtonBuilder } = require('@discordjs/builders');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('help')
        .setDescription('Replies with Help!'),
    async execute(interaction, client) {
        const embed = new EmbedBuilder()
            .setTitle('Help')
            .setColor([255, 255, 255])
            .setThumbnail(client.user.displayAvatarURL({ dynamic: true }))
            .setDescription('This is a list of all the commands available to you.')
            .addFields({ name: 'Page 1', value: 'Help & Resources' })
            .addFields({ name: 'Page 2', value: 'Tools' })
            .addFields({ name: 'Page 3', value: 'Fun' })
            .addFields({ name: 'Page 4', value: 'Moderattion' })

        const embed2 = new EmbedBuilder()
            .setTitle('Help')
            .setColor([255, 255, 255])
            .setThumbnail(client.user.displayAvatarURL({ dynamic: true }))
            .setDescription('This is a list of all the commands available to you.')
            .addFields({ name: '/help', value: 'Do /help for this page' })
            .addFields({ name: '/ping', value: 'Do /ping to get the bot\'s ping' })
            .addFields({ name: '/invite', value: 'Do /invite to get the bot\'s invite link' })

        const embed3 = new EmbedBuilder()
            .setTitle('Help')
            .setColor([255, 255, 255])
            .setThumbnail(client.user.displayAvatarURL({ dynamic: true }))
            .setDescription('This is a list of all the commands available to you.')
            .addFields({ name: '/whois', value: 'Do /whois to get a the user information of a given user' })
            .addFields({ name: '/avatar', value: 'Do /avatar to get a the avatar of a given user' })
            .addFields({ name: '/serverinfo', value: 'Do /serverinfo to get the server information' })
            .addFields({ name: '/botinfo', value: 'Do /botinfo to get the bot information' })
            .addFields({ name: '/embed', value: 'Do /emebed to help you make a embed' })

        const embed4 = new EmbedBuilder()
            .setTitle('Help')
            .setColor([255, 255, 255])
            .setThumbnail(client.user.displayAvatarURL({ dynamic: true }))
            .setDescription('This is a list of all the commands available to you.')
            .addFields({ name: '/kick', value: 'Do /kick to kick a user' })
            .addFields({ name: '/ban', value: 'Do /ban to ban a user' })
            .addFields({ name: '/timeout', value: 'Do /timeout to timeout a user' })
            .addFields({ name: '/untimeout', value: 'Do /untimeout to untimeout a user' })
            .addFields({ name: '/clear', value: 'Do /clear to clear a given amount of messages' })

        const buttons = new ActionRowBuilder()
            .addComponents(
                new ButtonBuilder()
                    .setLabel('Page 1')
                    .setStyle('Primary')
                    .setCustomId('page1'),
                new ButtonBuilder()
                    .setLabel('Page 2')
                    .setStyle('Primary')
                    .setCustomId('page2'),
                new ButtonBuilder()
                    .setLabel('Page 3')
                    .setStyle('Primary')
                    .setCustomId('page3'),
                new ButtonBuilder()
                    .setLabel('Page 4')
                    .setStyle('Primary')
                    .setCustomId('page4')
            )

        const message = await interaction.reply({ embeds: [embed], components: [buttons] });
        const collector = await message.createMessageComponentCollector();

        collector.on('collect', async i => {
            if (i.customId === `page1`) {
                if (i.user.id !== interaction.user.id) {
                    return await i.update({ content: `Only ${interaction.user.tag} can use these buttons!`, ephemeral: true });
                }
                await i.update({ embeds: [embed], components: [buttons] });
            }
            if (i.customId === `page2`) {
                if (i.user.id !== interaction.user.id) {
                    return await i.update({ content: `Only ${interaction.user.tag} can use these buttons!`, ephemeral: true });
                }
                await i.update({ embeds: [embed2], components: [buttons] });
            }
            if (i.customId === `page3`) {
                if (i.user.id !== interaction.user.id) {
                    return await i.update({ content: `Only ${interaction.user.tag} can use these buttons!`, ephemeral: true });
                }
                await i.update({ embeds: [embed3], components: [buttons] });
            }
            if (i.customId === `page4`) {
                if (i.user.id !== interaction.user.id) {
                    return await i.update({ content: `Only ${interaction.user.tag} can use these buttons!`, ephemeral: true });
                }
                await i.update({ embeds: [embed4], components: [buttons] });
            }
        });
    }
};
