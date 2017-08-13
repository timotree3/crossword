package main

import (
	"fmt"
	"io/ioutil"
	"encoding/json"
	"log"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"github.com/bwmarrin/discordgo"
)

var (
	nyc *time.Location
	chatChannels = []string{
		"today",
		"yesterday",
	}
	release_delay = time.Second * 30
)

const (
	TOPIC = `Dicussion of the mini crossword from %s.`
	WEEKDAY_HOUR = 12 + 10 // 10PM
	WEEKEND_HOUR = 12 + 6 // 6PM
)

func init() {
	var err error
  nyc, err = time.LoadLocation("America/New_York")
  if err != nil {
    log.Fatalln(err)
  }
}

func main() {
	Token, err := ioutil.ReadFile("token")
	if err != nil {
		log.Fatalln("error retrieving token from file:", err)
	}
	token := strings.TrimSpace(string(Token))
	// Create a new Discord session using the provided bot token.
	dg, err := discordgo.New("Bot " + token)
	if err != nil {
		log.Fatalln("error creating Discord session,", err)
	}

	dg.AddHandler(guildJoin)
	// Register the messageCreate func as a callback for MessageCreate events.
	dg.AddHandler(messageCreate)

	// Open a websocket connection to Discord and begin listening.
	err = dg.Open()
	if err != nil {
		log.Fatalln("error opening connection,", err)
	}

	if _, err := dg.User("@me"); err != nil {
		// Login unsuccessful
		log.Fatalln(err)
	}

	// Wait here until CTRL-C or other term signal is received.
	log.Println("Bot is now running.  Press CTRL-C to exit.")
	sc := make(chan os.Signal, 1)
	signal.Notify(sc, syscall.SIGINT, syscall.SIGTERM, os.Interrupt, os.Kill)
	<-sc

	// Cleanly close down the Discord session.
	dg.Close()
}

//Initialization code
func guildJoin(s *discordgo.Session, event *discordgo.GuildCreate) {

	if event.Guild.Unavailable {
		return
	}

	log.Printf("guild: %#v\n", event.Guild.Name)

	now := time.Now().In(nyc)

	// Update channels
	update(s, event.Guild, lastRelease(now))
}

// Daily update that runs every time a new crossword is released OR on startup
func update(s *discordgo.Session, guild *discordgo.Guild, now time.Time) error {
	for i, channelName := range chatChannels {
		channelID, ok := findChannel(guild, channelName)
		channel, err := s.Channel(channelID)
		// 	if the channel's date is...
		//    ...today's, do nothing
		// 		...yesterdays's, rename it to 'yesterday'
		// 		otherwise, copy it to 'archive'
		switch channel.Topic {
		case makeTopic(now):
			// if it was made today
			// don't do anything (it can't be out of date)
			continue
		case makeTopic(releaseTime(now.Add(24 * -time.Hour))):
			// if it was made yesterday
			// rename it to "#yesterday"
			s.ChannelEdit(channelID, "yesterday")
			return nil
		default:
			// otherwise
			archiveChannel(s, channelID)
			// archive it
		}
	}
	// if today is missing, create it
	return nil
}

// This function will be called (due to AddHandler above) every time a new
// message is created on any channel that the autenticated bot has access to.
func messageCreate(s *discordgo.Session, m *discordgo.MessageCreate) {
	log.Println("messageCreate")
	c, err := s.Channel(m.ChannelID)
	if err != nil {
		log.Println(err)
	}
	g, err := s.Guild(c.GuildID)
	if err != nil {
		log.Println(err)
	}
	log.Println(g.Name)

	// Ignore all messages created by the bot itself
	// This isn't required in this specific example but it's a good practice.
	if m.Author.ID == s.State.User.ID {
		return
	}
}

func send(s *discordgo.Session, channelID string, content string) (*discordgo.Message, error) {
	return s.ChannelMessageSend(channelID, "\u200B"+content)
}

func findChannel(guild *discordgo.Guild, name string) (channelID string, ok bool) {
	for _, channel := range guild.Channels {
		if channel.Name == name {
			channelID, ok = channel.ID, true
			return
		}
	}
	return
}

func makeTopic(t time.Time) string {
	return fmt.Sprintf(TOPIC, t.Format("1/2/2006"))
}

//-------FUNCTIONS FOR RELEASE TIMES-------
func releaseTime(t time.Time) time.Time {
	if t.Weekday() == time.Saturday || t.Weekday() == time.Sunday {
		return time.Date(t.Year(), t.Month(), t.Day(), WEEKEND_HOUR, 0, 0, 0, t.Location()).Add(release_delay)
	} else {
		return time.Date(t.Year(), t.Month(), t.Day(), WEEKDAY_HOUR, 0, 0, 0, t.Location()).Add(release_delay)
	}
}

func nextRelease(t time.Time) time.Time {
	if releaseTime(t).After(t) {
		// it has yet to be released today
		return releaseTime(t)
	} else {
		// return tomorrow's release time
		return releaseTime(t.Add(24 * time.Hour))
	}
}

func lastRelease(t time.Time) time.Time {
	if releaseTime(t).Before(t) {
		// it was already released today
		return releaseTime(t)
	} else {
		// return yesterday's release time
		return releaseTime(t.Add(24 * -time.Hour))
	}
}
/*
At any given time there are two things we need to be able to know.

When the next crossword will be released:
	func releaseTime(day of week) -> 6:00PM|10:00PM

	//we need to know: now, today's day of week
	if releaseTime(today) is after now:
		crossword will be released at releaseTime(today)
	else:
		crossword will be released at releaseTime(tomorrow)

When was the most recent crossword released:
	if releaseTime(today) is before now:
		crossword was released at releaseTime(today)
	else:
		crossword was released at releaseTime(yesterday)
*/
//--------------

func ChannelTopicEdit(s *discordgo.Session, channelID, topic string) (st *discordgo.Channel, err error) {

	data := struct {
		Topic string `json:"topic"`
	}{topic}

	body, err := s.RequestWithBucketID("PATCH", discordgo.EndpointChannel(channelID), data, discordgo.EndpointChannel(channelID))
	if err != nil {
		return
	}

	err = json.Unmarshal(body, &st)
	return
}


func archiveChannel(s *discordgo.Session, channelID string) {
// 	archiveContent :=
// 	s.ChannelMessageSend(archiveContent)
 	return
}

// Outputs a list of every message in the specified channel from newest to oldest
func readChannel(s *discordgo.Session, channelID string) (l []*discordgo.Message, err error) {
  const READ_LIMIT = 100
  new, err := s.ChannelMessages(channelID, READ_LIMIT, "", "", "")
  if err != nil {
    return
  }
  l = append(l, new...)

  for len(new) == READ_LIMIT {
    new, err = s.ChannelMessages(channelID, READ_LIMIT, l[len(l) - 1].ID, "", "")
    if err != nil {
      return
    }
    l = append(l, new...)
  }
  return
}
